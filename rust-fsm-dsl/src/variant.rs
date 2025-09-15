use std::fmt::Display;

use quote::ToTokens;
use syn::{parse::Parse, *};
/// Variant with no discriminator
#[derive(Hash, Debug, PartialEq, Eq)]
pub struct Variant {
    // attrs: Vec<Attribute>,
    ident: Ident,
    field: Option<(Type, Pat)>,
}

impl Parse for Variant {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        // let attrs = input.call(Attribute::parse_outer)?;
        // let _visibility: Visibility = input.parse()?;
        let ident: Ident = input.parse()?;
        let field = if input.peek(token::Paren) {
            let inp;
            parenthesized!(inp in input);
            let t = inp.parse()?;
            inp.parse::<Token![=>]>()?;
            Some((t, Pat::parse_multi(&inp)?))
        } else {
            None
        };
        Ok(Variant {
            // attrs,
            ident,
            field,
        })
    }
}

impl PartialOrd for Variant {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.ident.partial_cmp(&other.ident)
    }
}
impl Ord for Variant {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ident.cmp(&other.ident)
    }
}

impl ToTokens for Variant {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.ident.to_tokens(tokens);
        if let Some((t, _)) = &self.field {
            tokens.extend(quote::quote! { (#t) })
        }
    }
}

impl Variant {
    pub fn match_on(&self) -> proc_macro2::TokenStream {
        if let Self {
            ident,
            field: Some((_, p)),
        } = self
        {
            quote::quote! { #ident(#p) }
        } else {
            self.ident.to_token_stream()
        }
    }
}

impl Display for Variant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.match_on())
    }
}
