use {
    proc_macro::TokenStream,
    quote::quote,
    std::str::FromStr,
    syn::{parse_macro_input, AttributeArgs, Data, DeriveInput, ItemFn, Meta, NestedMeta},
};

#[proc_macro_attribute]
pub fn cw_derive(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attr as AttributeArgs);
    let input = parse_macro_input!(input as DeriveInput);

    let mut derive_serde = false;
    let mut derive_borsh = false;
    for attr in attrs {
        match attr {
            NestedMeta::Meta(Meta::Path(path)) if path.is_ident("serde") => {
                derive_serde = true;
            },
            NestedMeta::Meta(Meta::Path(path)) if path.is_ident("borsh") => {
                derive_borsh = true;
            },
            _ => {
                panic!("unsupported attribute, expecting `serde` or `borsh`");
            },
        }
    }

    let derives = match (derive_serde, derive_borsh) {
        (false, true) => quote! {
            #[derive(
                ::cw_std::__private::borsh::BorshSerialize,
                ::cw_std::__private::borsh::BorshDeserialize,
                ::std::clone::Clone,
                ::std::fmt::Debug,
                ::std::cmp::PartialEq,
                ::std::cmp::Eq,
            )]
        },
        (true, false) => quote! {
            #[::cw_std::__private::serde_with::skip_serializing_none]
            #[derive(
                ::cw_std::__private::serde::Serialize,
                ::cw_std::__private::serde::Deserialize,
                ::std::clone::Clone,
                ::std::fmt::Debug,
                ::std::cmp::PartialEq,
                ::std::cmp::Eq,
            )]
            #[serde(deny_unknown_fields, rename_all = "snake_case", crate = "::cw_std::__private::serde")]
        },
        (true, true) => quote! {
            #[::cw_std::__private::serde_with::skip_serializing_none]
            #[derive(
                ::cw_std::__private::serde::Serialize,
                ::cw_std::__private::serde::Deserialize,
                ::cw_std::__private::borsh::BorshSerialize,
                ::cw_std::__private::borsh::BorshDeserialize,
                ::std::clone::Clone,
                ::std::fmt::Debug,
                ::std::cmp::PartialEq,
                ::std::cmp::Eq,
            )]
            #[serde(deny_unknown_fields, rename_all = "snake_case", crate = "::cw_std::__private::serde")]
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
pub fn entry_point(_attr: TokenStream, mut item: TokenStream) -> TokenStream {
    let cloned = item.clone();
    let function = parse_macro_input!(cloned as ItemFn);
    let name = function.sig.ident.to_string();
    let args = function.sig.inputs.len();

    // e.g. "ptr0: usize, ptr1: usize, ptr2: usize, "
    let typed_ptrs = (0..args).fold(String::new(), |acc, i| format!("{acc}ptr{i}: usize, "));
    // e.g. "ptr0, ptr1, ptr2, "
    let ptrs = (0..args).fold(String::new(), |acc, i| format!("{acc}ptr{i}, "));

    // new module to avoid conflict of function names
    let new_code = format!(r##"
        #[cfg(target_arch = "wasm32")]
        mod __wasm_export_{name} {{
            #[no_mangle]
            extern "C" fn {name}({typed_ptrs}) -> usize {{
                cw_std::do_{name}(&super::{name}, {ptrs})
            }}
        }}
    "##);

    let entry = TokenStream::from_str(&new_code).unwrap();
    item.extend(entry);
    item
}
