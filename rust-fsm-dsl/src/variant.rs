use std::fmt::Display;

use quote::ToTokens;
use syn::{parse::Parse, *};
/// Variant with no discriminator
#[derive(Hash, Debug, PartialEq, Eq)]
pub struct Variant {
    pub ident: Ident,
    field: Option<(Type, Pat, Option<Expr>)>,
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

            Some((
                t,
                Pat::parse_multi(&inp)?,
                inp.lookahead1()
                    .peek(Token![if])
                    .then(|| inp.parse::<Token![if]>().and_then(|_| inp.parse::<Expr>()))
                    .transpose()?,
            ))
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
        if let Some((t, _, _)) = &self.field {
            tokens.extend(quote::quote! { (#t) })
        }
    }
}

impl Variant {
    pub fn match_on(&self) -> proc_macro2::TokenStream {
        if let Self {
            ident,
            field: Some((_, p, g)),
        } = self
        {
            let b = g
                .as_ref()
                .map_or_else(Default::default, |x| quote::quote! { if #x });
            quote::quote! { #ident(#p) #b }
        } else {
            self.ident.to_token_stream()
        }
    }
    pub fn separate(&self) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
        if let Self {
            ident,
            field: Some((_, p, g)),
        } = self
        {
            let b = g
                .as_ref()
                .map(|x| quote::quote! { if #x })
                .unwrap_or_default();
            (quote::quote! { #ident(#p) }, b)
        } else {
            (self.ident.to_token_stream(), quote::quote! {})
        }
    }
}

impl Display for Variant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ident)
    }
}
/// type and expression
#[derive(Hash, Debug, PartialEq, Eq, Clone)]
pub struct Final {
    pub ident: Ident,
    field: Option<(Type, Expr)>,
}

impl Parse for Final {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let ident: Ident = input.parse()?;
        let field = if input.peek(token::Paren) {
            let inp;
            parenthesized!(inp in input);
            let t = inp.parse()?;
            inp.parse::<Token![=>]>()?;

            Some((t, inp.parse()?))
        } else {
            None
        };
        Ok(Final {
            // attrs,
            ident,
            field,
        })
    }
}

impl PartialOrd for Final {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.ident.partial_cmp(&other.ident)
    }
}
impl Ord for Final {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ident.cmp(&other.ident)
    }
}

impl ToTokens for Final {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.ident.to_tokens(tokens);
        if let Some((v, _)) = &self.field {
            tokens.extend(quote::quote! { (#v) })
        }
    }
}

impl Final {
    pub fn reduce(&self) -> proc_macro2::TokenStream {
        if let Self {
            ident,
            field: Some((_, v)),
        } = self
        {
            quote::quote! { #ident ( #v ) }
        } else {
            self.ident.to_token_stream()
        }
    }
    pub fn variant(self) -> Variant {
        Variant {
            ident: self.ident,
            field: self.field.map(|(x, _)| {
                (
                    x,
                    Pat::Wild(PatWild {
                        attrs: vec![],
                        underscore_token: Default::default(),
                    }),
                    None,
                )
            }),
        }
    }
}

impl Display for Final {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ident)
    }
}
