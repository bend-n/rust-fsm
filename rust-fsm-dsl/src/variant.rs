use std::{collections::BTreeSet, fmt::Display};

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{parse::Parse, *};
/// Variant with no discriminator
#[derive(Debug, Clone)]
pub struct Variant {
    pub ident: Ident,
    pub field: Option<(Option<Type>, Pat, Option<Expr>)>,
}

pub fn find_type(of: &Variant, list: &[Variant]) -> Option<Type> {
    of.field.clone().and_then(|(x, _, _)| {
        x.or_else(|| {
            let i = &of.ident;
            list.iter()
                .filter(|x| &x.ident == i)
                .find_map(|x| x.field.as_ref().and_then(|x| x.0.clone()))
        })
    })
}
pub fn tokenize(
    inputs: &[Variant],
    f: impl FnOnce(Vec<TokenStream>) -> TokenStream,
) -> TokenStream {
    let (Ok(x) | Err(x)) = BTreeSet::from_iter(inputs)
        .into_iter()
        .map(|x| {
            let i = &x.ident;
            x.field.as_ref().map_or(Ok(quote::quote! { #i }), |_| {
                let y = find_type(x, inputs);
                y.ok_or(Error::new_spanned(&x.ident, "type never specified"))
                    .map(|y| quote::quote! {#i(#y)})
            })
        })
        .collect::<Result<_>>()
        .map_err(Error::into_compile_error)
        .map(f);
    x
}

impl Parse for Variant {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        // let attrs = input.call(Attribute::parse_outer)?;
        // let _visibility: Visibility = input.parse()?;
        let ident: Ident = input.parse()?;
        let field = if input.peek(token::Paren) {
            let inp;
            parenthesized!(inp in input);
            let ty = inp
                .to_string()
                .contains("=>")
                .then(|| {
                    inp.parse()
                        .and_then(|x| inp.parse::<Token![=>]>().map(|_| x))
                })
                .transpose()?;

            Some((
                ty,
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

impl PartialEq for Variant {
    fn eq(&self, other: &Self) -> bool {
        self.ident == other.ident
    }
}
impl Eq for Variant {}

impl PartialOrd for Variant {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Variant {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ident.cmp(&other.ident)
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
    pub fn separate(&self) -> (proc_macro2::TokenStream, Option<proc_macro2::TokenStream>) {
        if let Self {
            ident,
            field: Some((_, p, g)),
        } = self
        {
            let b = g.as_ref().map(|x| quote::quote! { #x });
            (quote::quote! { #ident(#p) }, b)
        } else {
            (self.ident.to_token_stream(), None)
        }
    }
}

impl Display for Variant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ident)
    }
}
/// type and expression
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Final(Option<Variant>);

impl Parse for Final {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let b = input.peek(Token![_]);
        b.then(|| input.parse::<Token![_]>()).transpose()?;
        (!b).then(|| {
            let ident: Ident = input.parse()?;
            let field = if input.peek(token::Paren) {
                let inp;
                parenthesized!(inp in input);
                let t = inp
                    .to_string()
                    .contains("=>")
                    .then(|| {
                        inp.parse()
                            .and_then(|x| inp.parse::<Token![=>]>().map(|_| x))
                    })
                    .transpose()?;

                Some((
                    t,
                    Pat::Wild(PatWild {
                        attrs: vec![],
                        underscore_token: Default::default(),
                    }),
                    Some(inp.parse()?),
                ))
            } else {
                None
            };
            Ok(Variant { ident, field })
        })
        .transpose()
        .map(Self)
    }
}

impl Final {
    pub fn reduce(&self) -> Option<proc_macro2::TokenStream> {
        self.0.as_ref().map(|x: &Variant| {
            if let Variant {
                ident,
                field: Some((_, _, v)),
            } = x
            {
                quote::quote! { #ident ( #v ) }
            } else {
                x.ident.to_token_stream()
            }
        })
    }
    pub fn variant(self) -> Option<Variant> {
        self.0
    }
}

impl Display for Final {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0.as_ref().map(|x| x.to_string()).unwrap_or("_".into())
        )
    }
}
