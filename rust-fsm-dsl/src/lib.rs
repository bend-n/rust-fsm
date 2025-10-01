//! DSL implementation for defining finite state machines for `rust-fsm`. See
//! more in the `rust-fsm` crate documentation.

#![recursion_limit = "128"]
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::*;
mod parser;
mod variant;
use variant::Variant;

use crate::{parser::StateMachineDef, variant::Final};
/// The full information about a state transition. Used to unify the
/// represantion of the simple and the compact forms.
struct Transition<'a> {
    initial_state: &'a Variant,
    input_value: &'a Variant,
    final_state: &'a Final,
    output: &'a Option<Final>,
}

fn attrs_to_token_stream(attrs: Vec<Attribute>) -> proc_macro2::TokenStream {
    let attrs = attrs.into_iter().map(ToTokens::into_token_stream);
    attrs.collect()
}

#[proc_macro]
/// Produce a state machine definition from the provided `rust-fmt` DSL
/// description.
pub fn state_machine(tokens: TokenStream) -> TokenStream {
    let StateMachineDef {
        doc,
        visibility,
        state_name,
        input_name,
        output_name,
        transitions,
        attributes,
    } = parse_macro_input!(tokens as parser::StateMachineDef);

    let doc = attrs_to_token_stream(doc);
    let attrs = attrs_to_token_stream(attributes);

    if transitions.is_empty() {
        let output = quote! {
            compile_error!("rust-fsm: at least one state transition must be provided");
        };
        return output.into();
    }

    let transitions = transitions.iter().flat_map(|def| {
        def.transitions.iter().map(move |transition| Transition {
            initial_state: &def.initial_state,
            input_value: &transition.input_value,
            final_state: &transition.final_state,
            output: &transition.output,
        })
    });
    // fn id(x: impl std::hash::Hash) -> u64 {
    //     use std::hash::BuildHasher;
    //     rustc_hash::FxSeededState::with_seed(5).hash_one(x)
    // }
    let mut states = vec![];
    let mut inputs = vec![];
    let mut outputs = vec![];
    let mut transition_cases = vec![];

    #[cfg(feature = "diagram")]
    let mut mermaid_diagram = format!(
        "///```mermaid
///stateDiagram-v2\n",
    );
    for transition in transitions {
        let Transition {
            initial_state,
            final_state,
            input_value,
            output,
        } = transition;

        // #[cfg(feature = "diagram")]
        // writeln!(
        //     mermaid_diagram,
        //     "///    {}: {initial_state}",
        //     id(&initial_state)
        // )
        // .unwrap();
        // #[cfg(feature = "diagram")]
        // writeln!(
        //     mermaid_diagram,
        //     "///    {}: {final_state}",
        //     id(&final_state)
        // )
        // .unwrap();
        use std::fmt::Write;
        #[cfg(feature = "diagram")]
        write!(
            mermaid_diagram,
            "///    {}",
            &format!(
                "{:?}",
                format!(
                    "{initial_state} --> {final_state}: {}",
                    input_value.match_on()
                )
            )
            .trim_matches('"'),
        )
        .unwrap();

        let (initial_, guard_) = initial_state.separate();
        let final_ = final_state
            .reduce()
            .map_or(initial_state.match_on(), |x| quote! { #x });
        let (input_, guard) = input_value.separate();
        let guard = guard_
            .clone()
            .zip(guard.clone())
            .map(|(x, y)| {
                quote! { if #x && #y }
            })
            .or(guard_.or(guard).map(|x| quote! { if #x }))
            .unwrap_or_default();

        // let input_ = input_value.match_on();
        // let final_state_ = final_state.match_on();
        let output_ = output
            .as_ref()
            .map(|x| {
                #[cfg(feature = "diagram")]
                mermaid_diagram.push_str(&format!(" [\"{x}\"]"));
                let output = x.reduce().unwrap();
                quote! { ::core::option::Option::Some(Self::Output::#output) }
            })
            .unwrap_or(quote! { ::core::option::Option::None });
        // let x = format!("{}, {} {} => {}", initial_, input_, guard, output_);
        transition_cases.push(quote! {
            (Self::#initial_, Self::Input::#input_) #guard => {
                ::core::result::Result::Ok((Self::#final_, #output_))
            }
        });

        #[cfg(feature = "diagram")]
        mermaid_diagram.push('\n');

        states.push(initial_state.clone());
        states.extend(final_state.clone().variant());
        inputs.push(input_value.clone());
        if let Some(output) = output {
            outputs.push(output.clone().variant().unwrap());
        }
    }

    #[cfg(feature = "diagram")]
    mermaid_diagram.push_str("///```");
    #[cfg(feature = "diagram")]
    let mermaid_diagram: proc_macro2::TokenStream = mermaid_diagram
        .replace("::", "#58;#58;")
        .replace('(', "#40;")
        .replace(')', "#41;")
        .replace('[', "#91;")
        .replace(']', "#93;")
        .replace('|', "#124;")
        .replace("Default", "def")
        .parse()
        .unwrap();

    let input_impl = variant::tokenize(&inputs, |x| {
        input_name.tokenize(|f| {
            quote! {
                #attrs
                #visibility enum #f {
                    #(#x),*
                }
            }
        })
    });
    let input_name = input_name.path();
    let state_impl = variant::tokenize(&states, |x| {
        state_name.tokenize(|f| {
            quote! {
                #attrs
                #visibility enum #f {
                    #(#x),*
                }
            }
        })
    });
    let state_name = state_name.path();
    let output_impl = variant::tokenize(&outputs, |outputs| {
        output_name.tokenize(|output_name| {
            // Many attrs and derives may work incorrectly (or simply not work) for empty enums, so we just skip them
            // altogether if the output alphabet is empty.
            let attrs = if outputs.is_empty() {
                quote!()
            } else {
                attrs.clone()
            };

            quote! {
                #attrs
                #visibility enum #output_name {
                    #(#outputs),*
                }
            }
        })
    });

    let output_name = output_name.path();

    #[cfg(feature = "diagram")]
    let diagram = quote! {
        #[cfg_attr(doc, ::rust_fsm::aquamarine)]
        #mermaid_diagram
    };

    #[cfg(not(feature = "diagram"))]
    let diagram = quote!();
    let output = quote! {
        #input_impl
        #doc
        #diagram
        #state_impl
        #output_impl

        impl ::rust_fsm::StateMachineImpl for #state_name {
            type Input = #input_name;
            type Output = #output_name;

            fn transition(self, input: Self::Input) -> ::core::result::Result<
                (Self, ::core::option::Option<Self::Output>),
                ::rust_fsm::TransitionImpossibleError<Self, Self::Input>
            > {
                match (self, input) {
                    #(#transition_cases)*
                    (state, input) => ::core::result::Result::Err(::rust_fsm::TransitionImpossibleError { state, input, }),
                }
            }

        }

    };

    output.into()
}
