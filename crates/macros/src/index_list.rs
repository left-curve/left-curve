use {
    proc_macro::TokenStream,
    quote::quote,
    std::panic,
    syn::{parse_macro_input, ItemStruct},
};

macro_rules! get_ident {
    ($ty:ident, $s:expr, $worng_token_tree_err:expr) => {{
        if let proc_macro2::TokenTree::$ty(ty) = $s.expect($worng_token_tree_err) {
            ty
        } else {
            panic!($worng_token_tree_err);
        }
    }};
}

pub fn process(attrs: TokenStream, item: TokenStream) -> TokenStream {
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
