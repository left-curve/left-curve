use {
    proc_macro::TokenStream,
    quote::quote,
    std::{panic, str::FromStr},
    syn::{
        parse::{Parse, ParseStream},
        parse_macro_input, Data, DeriveInput, Ident, ItemFn, ItemStruct, Token,
    },
};

struct DeriveArgs {
    serde: bool,
    borsh: bool,
}

impl Parse for DeriveArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut serde = false;
        let mut borsh = false;

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
                _ => {
                    return Err(input.error("unsupported attribute, expecting `serde` or `borsh`"));
                },
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(DeriveArgs { borsh, serde })
    }
}

#[proc_macro_attribute]
pub fn derive(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attr as DeriveArgs);
    let input = parse_macro_input!(input as DeriveInput);

    let derives = match (attrs.serde, attrs.borsh) {
        (false, true) => quote! {
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
        (true, false) => quote! {
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
        (true, true) => quote! {
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

#[proc_macro_attribute]
pub fn export(_attr: TokenStream, mut item: TokenStream) -> TokenStream {
    let cloned = item.clone();
    let function = parse_macro_input!(cloned as ItemFn);
    let name = function.sig.ident.to_string();
    let args = function.sig.inputs.len();

    // E.g. "ptr0: usize, ptr1: usize, ptr2: usize, "
    let typed_ptrs = (0..args).fold(String::new(), |acc, i| format!("{acc}ptr{i}: usize, "));
    // E.g. "ptr0, ptr1, ptr2, "
    let ptrs = (0..args).fold(String::new(), |acc, i| format!("{acc}ptr{i}, "));

    // New module to avoid conflict of function names
    let new_code = format!(
        r##"
            #[cfg(target_arch = "wasm32")]
            mod __wasm_export_{name} {{
                #[no_mangle]
                extern "C" fn {name}({typed_ptrs}) -> usize {{
                    grug::do_{name}(&super::{name}, {ptrs})
                }}
            }}
        "##
    );

    let entry = TokenStream::from_str(&new_code).unwrap();
    item.extend(entry);
    item
}

macro_rules! get_ident {
    ($ty:ident, $s:expr, $worng_token_tree_err:expr) => {{
        if let proc_macro2::TokenTree::$ty(ty) = $s.expect($worng_token_tree_err) {
            ty
        } else {
            panic!($worng_token_tree_err);
        }
    }};
}

/// Implement the trait `IndexList` for a Index Struct:
/// ```rust ignore
/// #[grug::derive(borsh)]
/// pub struct Test {
///    pub foo: String,
///    pub bar: u64,
/// }
///
/// #[grug::index_list(Test, u64)]
/// pub struct TestIndexes<'a> {
///    pub foo: MultiIndex<'a, u64, String, Test, Borsh>,
///    pub bar: UniqueIndex<'a, u64, Test, Borsh>,
/// }
/// ```
#[proc_macro_attribute]
pub fn index_list(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);

    let mut attrs = proc_macro2::TokenStream::from(attrs).into_iter();

    let ty = get_ident!(
        Ident,
        attrs.next(),
        "Main struct type is missing, expecting the following format: `#[index_list(Struct, PK)]"
    );
    // Check if the next token is a comma
    get_ident!(
        Punct,
        attrs.next(),
        "Comma is missing on: expecting the following format: `#[index_list(Struct, PK)]`"
    );

    // Transform the remaining attributes into a TokenStream.
    // This will interpret the remaining attributes as the primary key.
    let pk = proc_macro2::TokenStream::from_iter(attrs);

    let struct_ty = input.ident.clone();

    let names = input
        .fields
        .clone()
        .into_iter()
        .map(|e| {
            let name = e.ident.unwrap();
            quote! { &self.#name }
        })
        .collect::<Vec<_>>();

    let expanded = quote! {
        #input

        impl grug::IndexList<#pk, #ty> for #struct_ty<'_> {
            fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn grug::Index<#pk, #ty>> + '_> {
                let v: Vec<&dyn grug::Index<#pk, #ty>> = vec![#(#names),*];
                Box::new(v.into_iter())
            }
        }
    };

    TokenStream::from(expanded)
}
