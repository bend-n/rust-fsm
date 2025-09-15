//! DSL implementation for defining finite state machines for `rust-fsm`. See
//! more in the `rust-fsm` crate documentation.

#![recursion_limit = "128"]
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use std::{collections::BTreeSet, iter::FromIterator};
use syn::*;
mod parser;
mod variant;
use variant::Variant;

use crate::parser::StateMachineDef;
/// The full information about a state transition. Used to unify the
/// represantion of the simple and the compact forms.
struct Transition<'a> {
    initial_state: &'a Variant,
    input_value: &'a Variant,
    final_state: &'a Variant,
    output: &'a Option<Variant>,
}

fn attrs_to_token_stream(attrs: Vec<Attribute>) -> proc_macro2::TokenStream {
    let attrs = attrs.into_iter().map(ToTokens::into_token_stream);
    proc_macro2::TokenStream::from_iter(attrs)
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
        initial_state,
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

    let mut states = BTreeSet::new();
    let mut inputs = BTreeSet::new();
    let mut outputs = BTreeSet::new();
    let mut transition_cases = Vec::new();
    let mut output_cases = Vec::new();

    #[cfg(feature = "diagram")]
    let mut mermaid_diagram = format!(
        "///```mermaid\n///stateDiagram-v2\n///    [*] --> {}\n",
        initial_state
    );

    states.insert(&initial_state);

    for transition in transitions {
        let Transition {
            initial_state,
            final_state,
            input_value,
            output,
        } = transition;

        #[cfg(feature = "diagram")]
        mermaid_diagram.push_str(&format!(
            "///    {initial_state} --> {final_state}: {input_value}"
        ));

        let input_ = input_value.match_on();
        let final_state_ = final_state.match_on();
        transition_cases.push(quote! {
            (Self::#initial_state, Self::Input::#input_) => {
                Some(Self::#final_state_)
            }
        });

        if let Some(output_value) = output {
            let output_value_ = output_value.match_on();
            output_cases.push(quote! {
                (Self::#initial_state, Self::Input::#input_) => {
                    Some(Self::Output::#output_value_)
                }
            });

            #[cfg(feature = "diagram")]
            mermaid_diagram.push_str(&format!(" [{output_value}]"));
        }

        #[cfg(feature = "diagram")]
        mermaid_diagram.push('\n');

        states.insert(initial_state);
        states.insert(final_state);
        inputs.insert(input_value);
        if let Some(ref output) = output {
            outputs.insert(output);
        }
    }

    #[cfg(feature = "diagram")]
    mermaid_diagram.push_str("///```");
    #[cfg(feature = "diagram")]
    let mermaid_diagram: proc_macro2::TokenStream = mermaid_diagram.parse().unwrap();

    let initial_state_name = &initial_state;

    let input_impl = input_name.tokenize(|f| {
        quote! {
            #attrs
            #visibility enum #f {
                #(#inputs),*
            }
        }
    });
    let input_name = input_name.path();
    let state_impl = state_name.tokenize(|f| {
        quote! {
            #attrs
            #visibility enum #f {
                #(#states),*
            }
        }
    });
    let state_name = state_name.path();

    let output_impl = output_name.tokenize(|output_name| {
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
            const INITIAL_STATE: Self = Self::#initial_state_name;

            fn transition(self, input: Self::Input) -> Option<Self> {
                match (self, input) {
                    #(#transition_cases)*
                    _ => None,
                }
            }

            fn output(self, input: Self::Input) -> Option<Self::Output> {
                match (self, input) {
                    #(#output_cases)*
                    _ => None,
                }
            }
        }

    };

    output.into()
}
