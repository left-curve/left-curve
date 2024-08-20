use {
    proc_macro::TokenStream,
    quote::quote,
    std::panic,
    syn::{
        parse::{Parse, ParseStream},
        parse_macro_input,
        token::Comma,
        Data, DeriveInput, Ident,
    },
};

struct Args {
    serde: bool,
    borsh: bool,
    ts: bool,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut serde = false;
        let mut borsh = false;
        let mut ts = false;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;

            match ident.to_string().as_str() {
                "serde" if serde => {
                    return Err(input.error("don't input `serde` attribute twice"));
                },
                "serde" if !serde => {
                    serde = true;
                },
                "borsh" if borsh => {
                    return Err(input.error("don't input `borsh` attribute twice"));
                },
                "borsh" if !borsh => {
                    borsh = true;
                },
                "ts" if ts => {
                    return Err(input.error("don't input `ts` attribute twice"));
                },
                "ts" if !ts => {
                    ts = true;
                },
                _ => {
                    return Err(input.error("unsupported attribute, expecting `serde` or `borsh`"));
                },
            }

            if !input.is_empty() {
                input.parse::<Comma>()?;
            }
        }

        Ok(Args { borsh, serde, ts })
    }
}

pub fn process(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attr as Args);
    let input = parse_macro_input!(input as DeriveInput);

    let derives = match (attrs.serde, attrs.borsh, attrs.ts) {
        (false, true, false) => quote! {
            #[derive(
                ::grug::__private::borsh::BorshSerialize,
                ::grug::__private::borsh::BorshDeserialize,
                ::std::clone::Clone,
                ::std::fmt::Debug,
                ::std::cmp::PartialEq,
                ::std::cmp::Eq,
            )]
            #[borsh(crate = "::grug::__private::borsh")]
        },
        (true, false, false) => quote! {
            #[::grug::__private::serde_with::skip_serializing_none]
            #[derive(
                ::grug::__private::serde::Serialize,
                ::grug::__private::serde::Deserialize,
                ::std::clone::Clone,
                ::std::fmt::Debug,
                ::std::cmp::PartialEq,
                ::std::cmp::Eq,
            )]
            #[serde(deny_unknown_fields, rename_all = "snake_case", crate = "::grug::__private::serde")]
        },
        (true, true, false) => quote! {
            #[::grug::__private::serde_with::skip_serializing_none]
            #[derive(
                ::grug::__private::serde::Serialize,
                ::grug::__private::serde::Deserialize,
                ::grug::__private::borsh::BorshSerialize,
                ::grug::__private::borsh::BorshDeserialize,
                ::std::clone::Clone,
                ::std::fmt::Debug,
                ::std::cmp::PartialEq,
                ::std::cmp::Eq,
            )]
            #[serde(deny_unknown_fields, rename_all = "snake_case", crate = "::grug::__private::serde")]
            #[borsh(crate = "::grug::__private::borsh")]
        },
        (false, true, true) => quote! {
            #[derive(
                ::grug::__private::borsh::BorshSerialize,
                ::grug::__private::borsh::BorshDeserialize,
                ::std::clone::Clone,
                ::std::fmt::Debug,
                ::std::cmp::PartialEq,
                ::std::cmp::Eq,
                ::ts_rs::TS,
            )]
            #[borsh(crate = "::grug::__private::borsh")]
            #[ts(export)]
        },
        (true, false, true) => quote! {
            #[::grug::__private::serde_with::skip_serializing_none]
            #[derive(
                ::grug::__private::serde::Serialize,
                ::grug::__private::serde::Deserialize,
                ::std::clone::Clone,
                ::std::fmt::Debug,
                ::std::cmp::PartialEq,
                ::std::cmp::Eq,
                ::ts_rs::TS,
            )]
            #[serde(deny_unknown_fields, rename_all = "snake_case", crate = "::grug::__private::serde")]
            #[ts(export)]
        },
        (true, true, true) => quote! {
            #[::grug::__private::serde_with::skip_serializing_none]
            #[derive(
                ::grug::__private::serde::Serialize,
                ::grug::__private::serde::Deserialize,
                ::grug::__private::borsh::BorshSerialize,
                ::grug::__private::borsh::BorshDeserialize,
                ::std::clone::Clone,
                ::std::fmt::Debug,
                ::std::cmp::PartialEq,
                ::std::cmp::Eq,
                ::ts_rs::TS,
            )]
            #[serde(deny_unknown_fields, rename_all = "snake_case", crate = "::grug::__private::serde")]
            #[borsh(crate = "::grug::__private::borsh")]
            #[ts(export)]
        },
        _ => {
            panic!("unsupported attribute combination: expecting either `serde`, `borsh`, or both");
        },
    };

    match input.data {
        Data::Struct(_) | Data::Enum(_) => quote! {
            #derives
            #input
        },
        Data::Union(_) => {
            panic!("union is not supported; expecting Struct or Enum");
        },
    }
    .into()
}
