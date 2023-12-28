use {
    proc_macro::TokenStream,
    quote::quote,
    syn::{parse_macro_input, Data, DeriveInput},
};

#[proc_macro_attribute]
pub fn cw_serde(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match input.data {
        Data::Struct(_) | Data::Enum(_) => quote! {
            #[derive(
                ::cw_std::__private::serde::Serialize,
                ::cw_std::__private::serde::Deserialize,
                ::std::clone::Clone,
                ::std::fmt::Debug,
                ::std::cmp::PartialEq,
                ::std::cmp::Eq,
            )]
            #[serde(deny_unknown_fields, rename_all = "snake_case", crate = "::cw_std::__private::serde")]
            #input
        },
        Data::Union(_) => panic!("Union is not supported"),
    }
    .into()
}
