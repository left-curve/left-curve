use {
    proc_macro::TokenStream,
    quote::quote,
    syn::{
        Expr, Fields, ItemStruct,
        parse::{Parse, ParseStream},
        parse_macro_input,
        token::Comma,
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
    let names = match input.fields.clone() {
        Fields::Named(named) => {
            let mut names = Vec::new();
            for field in named.named {
                let Some(name) = field.ident else {
                    return syn::Error::new_spanned(field, "index field must be named")
                        .to_compile_error()
                        .into();
                };
                names.push(quote! { &self.#name });
            }
            names
        },
        _ => {
            return syn::Error::new_spanned(
                &input,
                "`index_list` requires a struct with named fields",
            )
            .to_compile_error()
            .into();
        },
    };

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
