use super::variant::Variant;
use syn::{
    parse::{Error, Parse, ParseStream, Result},
    token::Bracket,
    *,
};
/// The output of a state transition
pub struct Output(Option<Variant>);

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

impl From<Output> for Option<Variant> {
    fn from(output: Output) -> Self {
        output.0
    }
}

/// Represents a part of state transition without the initial state. The `Parse`
/// trait is implemented for the compact form.
pub struct TransitionEntry {
    pub input_value: Variant,
    pub final_state: Variant,
    pub output: Option<Variant>,
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
        let initial_state = input.parse()?;
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
                    initial_state,
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
    pub name: Ident,
    pub initial_state: Variant,
    pub transitions: Vec<TransitionDef>,
    pub attributes: Vec<Attribute>,
    pub input_type: Option<Path>,
    pub state_type: Option<Path>,
    pub output_type: Option<Path>,
}

impl Parse for StateMachineDef {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut state_machine_attributes = Vec::new();
        let mut doc = Vec::new();
        let attributes = Attribute::parse_outer(input)?
            .into_iter()
            .filter_map(|attribute| {
                if attribute.path().is_ident("state_machine") {
                    state_machine_attributes.push(attribute);
                    None
                } else if attribute.path().is_ident("doc") {
                    doc.push(attribute);
                    None
                } else {
                    Some(attribute)
                }
            })
            .collect();

        let mut input_type = None;
        let mut state_type = None;
        let mut output_type = None;

        for attribute in state_machine_attributes {
            attribute.parse_nested_meta(|meta| {
                let content;
                parenthesized!(content in meta.input);
                let p: Path = content.parse()?;

                if meta.path.is_ident("input") {
                    input_type = Some(p);
                } else if meta.path.is_ident("state") {
                    state_type = Some(p);
                } else if meta.path.is_ident("output") {
                    output_type = Some(p);
                }

                Ok(())
            })?;
        }

        let visibility = input.parse()?;
        let name = input.parse()?;

        let initial_state_content;
        parenthesized!(initial_state_content in input);
        let initial_state = initial_state_content.parse()?;

        let transitions = input
            .parse_terminated(TransitionDef::parse, Token![,])?
            .into_iter()
            .collect();

        Ok(Self {
            doc,
            visibility,
            name,
            initial_state,
            transitions,
            attributes,
            input_type,
            state_type,
            output_type,
        })
    }
}
