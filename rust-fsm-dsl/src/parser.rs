use crate::variant::Final;

use super::variant::Variant;
use proc_macro2::TokenStream;
use syn::{
    parse::{Error, Parse, ParseStream, Result},
    token::Bracket,
    *,
};
/// The output of a state transition
pub struct Output(Option<Final>);

impl Parse for Output {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.lookahead1().peek(Bracket) {
            let output_content;
            bracketed!(output_content in input);
            Ok(Self(Some(output_content.parse()?)))
        } else {
            Ok(Self(None))
        }
    }
}

impl From<Output> for Option<Final> {
    fn from(output: Output) -> Self {
        output.0
    }
}

/// Represents a part of state transition without the initial state. The `Parse`
/// trait is implemented for the compact form.
pub struct TransitionEntry {
    pub input_value: Variant,
    pub final_state: Final,
    pub output: Option<Final>,
}

impl Parse for TransitionEntry {
    fn parse(input: ParseStream) -> Result<Self> {
        let input_value = input.parse()?;
        input.parse::<Token![=>]>()?;
        let final_state = input.parse()?;
        let output = input.parse::<Output>()?.into();
        Ok(Self {
            input_value,
            final_state,
            output,
        })
    }
}

/// Parses the transition in any of the possible formats.
pub struct TransitionDef {
    pub initial_state: Variant,
    pub transitions: Vec<TransitionEntry>,
}

impl Parse for TransitionDef {
    fn parse(input: ParseStream) -> Result<Self> {
        let initial_state: Variant = input.parse()?;
        input.parse::<Token![=>]>()?;
        // Parse the transition in the simple format
        // InitialState => Input => ResultState
        let transitions = if !input.lookahead1().peek(token::Brace) {
            let input_value = input.parse()?;
            input.parse::<Token![=>]>()?;
            let final_state = input.parse()?;
            let output = input.parse::<Output>()?.into();

            vec![TransitionEntry {
                input_value,
                final_state,
                output,
            }]
        } else {
            // Parse the transition in the compact format
            // InitialState => {
            //     Input1 => State1,
            //     Input2 => State2 [Output]
            // }
            let entries_content;
            braced!(entries_content in input);

            let entries: Vec<_> = entries_content
                .parse_terminated(TransitionEntry::parse, Token![,])?
                .into_iter()
                .collect();
            if entries.is_empty() {
                return Err(Error::new_spanned(
                    initial_state.ident,
                    "No transitions provided for a compact representation",
                ));
            }
            entries
        };
        Ok(Self {
            initial_state,
            transitions,
        })
    }
}

/// Parses the whole state machine definition in the following form (example):
///
/// ```rust,ignore
/// state_machine! {
///     CircuitBreaker(Closed)
///
///     Closed(Unsuccessful) => Open [SetupTimer],
///     Open(TimerTriggered) => HalfOpen,
///     HalfOpen => {
///         Successful => Closed,
///         Unsuccessful => Open [SetupTimer]
///     }
/// }
/// ```
pub struct StateMachineDef {
    pub doc: Vec<Attribute>,
    /// The visibility modifier (applies to all generated items)
    pub visibility: Visibility,
    pub state_name: ImplementationRequired,
    pub input_name: ImplementationRequired,
    pub output_name: ImplementationRequired,
    pub transitions: Vec<TransitionDef>,
    pub attributes: Vec<Attribute>,
}

pub enum ImplementationRequired {
    Yes(Ident, Generics),
    No(Path),
}

impl ImplementationRequired {
    pub fn tokenize(&self, f: impl Fn(&Ident) -> TokenStream) -> TokenStream {
        match self {
            ImplementationRequired::Yes(ident, _) => f(ident),
            ImplementationRequired::No(_) => TokenStream::default(),
        }
    }
    pub fn g(&self) -> TokenStream {
        match self {
            ImplementationRequired::Yes(_, g) => quote::quote! {#g},
            ImplementationRequired::No(_) => TokenStream::default(),
        }
    }
    pub fn path(self) -> Path {
        match self {
            ImplementationRequired::Yes(ident, _) => ident.into(),
            ImplementationRequired::No(path) => path,
        }
    }
}

impl Parse for StateMachineDef {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut doc = Vec::new();
        let attributes = Attribute::parse_outer(input)?
            .into_iter()
            .filter_map(|attribute| {
                if attribute.path().is_ident("doc") {
                    doc.push(attribute);
                    None
                } else {
                    Some(attribute)
                }
            })
            .collect();

        let visibility = input.parse()?;
        let i = || {
            input
                .peek(Token![::])
                .then(|| {
                    input.parse::<Path>().map(|mut x| {
                        x.leading_colon = None;
                        ImplementationRequired::No(x)
                    })
                })
                .unwrap_or_else(|| {
                    let t = input.parse::<Ident>()?;
                    let g = input.parse::<Generics>()?;
                    Ok(ImplementationRequired::Yes(t, g))
                })
        };
        let state_name = i()?;
        input.parse::<Token![=>]>()?;
        let input_name = i()?;
        input.parse::<Token![=>]>()?;
        let output_name = i()?;

        let transitions = input
            .parse_terminated(TransitionDef::parse, Token![,])?
            .into_iter()
            .collect();

        Ok(Self {
            doc,
            visibility,
            state_name,
            input_name,
            output_name,
            transitions,
            attributes,
        })
    }
}
