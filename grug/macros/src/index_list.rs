use {
    proc_macro::TokenStream,
    quote::quote,
    syn::{
        parse::{Parse, ParseStream},
        parse_macro_input,
        token::Comma,
        Expr, ItemStruct,
    },
};

struct Args {
    /// Type of the `IndexedMap`'s primary key.
    pk: Expr,
    /// Type of the `IndexedMap`'s value.
    ty: Expr,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let pk = input.parse()?;
        input.parse::<Comma>()?;
        let ty = input.parse()?;

        Ok(Self { pk, ty })
    }
}

pub fn process(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let Args { pk, ty } = parse_macro_input!(attrs as Args);
    let input = parse_macro_input!(item as ItemStruct);

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

    quote! {
        #input

        impl ::grug::IndexList<#pk, #ty> for #struct_ty<'_> {
            fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn ::grug::Index<#pk, #ty>> + '_> {
                let v: Vec<&dyn ::grug::Index<#pk, #ty>> = vec![#(#names),*];
                Box::new(v.into_iter())
            }
        }
    }
    .into()
}
