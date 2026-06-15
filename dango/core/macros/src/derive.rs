use {
    proc_macro::TokenStream,
    quote::quote,
    std::panic,
    syn::{
        Data, DeriveInput, Ident,
        parse::{Parse, ParseStream},
        parse_macro_input,
        token::Comma,
    },
};

struct Args {
    serde: bool,
    borsh: bool,
    query: bool,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut serde = false;
        let mut borsh = false;
        let mut query = false;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;

            match ident.to_string().as_str() {
                "Serde" if serde => {
                    return Err(input.error("don't input `Serde` attribute twice"));
                },
                "Serde" if !serde => {
                    serde = true;
                },
                "Borsh" if borsh => {
                    return Err(input.error("don't input `Borsh` attribute twice"));
                },
                "Borsh" if !borsh => {
                    borsh = true;
                },
                "QueryRequest" if query => {
                    return Err(input.error("don't input `QueryRequest` attribute twice"));
                },
                "QueryRequest" if !query => {
                    query = true;
                },
                _ => {
                    return Err(input.error(
                        "unsupported attribute, expecting `Serde`, `Borsh` or `QueryRequest`",
                    ));
                },
            }

            if !input.is_empty() {
                input.parse::<Comma>()?;
            }
        }

        Ok(Args {
            borsh,
            serde,
            query,
        })
    }
}

pub fn process(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attr as Args);
    let input = parse_macro_input!(input as DeriveInput);

    let derives = match (attrs.serde, attrs.borsh) {
        (false, true) => quote! {
            #[derive(
                ::dango_primitives::__private::borsh::BorshSerialize,
                ::dango_primitives::__private::borsh::BorshDeserialize,
                ::std::clone::Clone,
                ::std::fmt::Debug,
                ::std::cmp::PartialEq,
                ::std::cmp::Eq,
            )]
            #[borsh(crate = "::dango_primitives::__private::borsh")]
        },
        (true, false) => quote! {
            #[::dango_primitives::__private::serde_with::skip_serializing_none]
            #[derive(
                ::dango_primitives::__private::serde::Serialize,
                ::dango_primitives::__private::serde::Deserialize,
                ::std::clone::Clone,
                ::std::fmt::Debug,
                ::std::cmp::PartialEq,
                ::std::cmp::Eq,
            )]
            #[serde(rename_all = "snake_case", crate = "::dango_primitives::__private::serde")]
        },
        (true, true) => quote! {
            #[::dango_primitives::__private::serde_with::skip_serializing_none]
            #[derive(
                ::dango_primitives::__private::serde::Serialize,
                ::dango_primitives::__private::serde::Deserialize,
                ::dango_primitives::__private::borsh::BorshSerialize,
                ::dango_primitives::__private::borsh::BorshDeserialize,
                ::std::clone::Clone,
                ::std::fmt::Debug,
                ::std::cmp::PartialEq,
                ::std::cmp::Eq,
            )]
            #[serde(rename_all = "snake_case", crate = "::dango_primitives::__private::serde")]
            #[borsh(crate = "::dango_primitives::__private::borsh")]
        },
        (false, false) => quote! {
            #[derive(
                ::std::clone::Clone,
                ::std::fmt::Debug,
                ::std::cmp::PartialEq,
                ::std::cmp::Eq,
            )]
        },
    };

    let query_derive = if attrs.query {
        quote! {
            #[derive(::dango_primitives::QueryRequest)]
        }
    } else {
        quote! {}
    };

    match input.data {
        Data::Struct(_) | Data::Enum(_) => quote! {
            #derives
            #query_derive
            #input
        },
        Data::Union(_) => {
            panic!("union is not supported; expecting Struct or Enum");
        },
    }
    .into()
}
