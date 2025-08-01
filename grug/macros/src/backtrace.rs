use {
    proc_macro::TokenStream,
    quote::quote,
    syn::{Data, DeriveInput, Fields, Ident, Path, parse::Parse, parse_macro_input, parse_quote},
};

struct InputArgs {
    crate_name: Path,
}

impl Parse for InputArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let crate_name = if input.is_empty() {
            syn::parse_quote!(grug_types_base)
        } else {
            input.parse()?
        };

        if !input.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "only one argument is allowed",
            ));
        }

        Ok(InputArgs { crate_name })
    }
}

pub fn process(attr: TokenStream, input: TokenStream) -> TokenStream {
    let crate_path = parse_macro_input!(attr as InputArgs).crate_name;

    let mut input = parse_macro_input!(input as DeriveInput);

    let input_ident = &input.ident;

    let mut impl_from = vec![];

    let mut match_statement = vec![];

    if let Data::Enum(en) = &mut input.data {
        for variant in en.variants.iter_mut() {
            let mut fresh = false;

            variant.attrs.retain(|a| {
                if a.path().is_ident("backtrace") {
                    let inner = a.parse_args::<Ident>().unwrap();

                    if inner == "fresh" {
                        fresh = true;
                    } else {
                        panic!("expected `fresh` attribute, got `{}`", inner);
                    }
                    false
                } else {
                    true
                }
            });

            let variant_ident = &variant.ident;

            match &mut variant.fields {
                Fields::Named(fields) => {
                    fields.named.push(parse_quote! {
                        backtrace: #crate_path::BT
                    });

                    match_statement.push(quote! {
                        Self::#variant_ident{backtrace,..} => backtrace.clone(),
                    });
                },
                Fields::Unnamed(unamed) => {
                    let mut iter = unamed.unnamed.iter_mut();
                    let field = iter.next().expect("no unnamed fields");
                    let original_ty = &field.ty.clone();
                    field.ty = parse_quote! { #crate_path::UnnamedBacktrace<#original_ty> };

                    if fresh {
                        impl_from.push(quote! {
                            impl From<#original_ty> for #input_ident {
                                fn from(t: #original_ty) -> Self {
                                    Self::#variant_ident(#crate_path::UnnamedBacktrace::new(t))
                                }
                            }
                        });
                    } else {
                        impl_from.push(quote! {
                            impl From<#original_ty> for #input_ident {
                                fn from(t: #original_ty) -> Self {
                                    let bt = #crate_path::Backtraceable::backtrace(&t);
                                    Self::#variant_ident(#crate_path::UnnamedBacktrace::new_with_bt(t, bt))
                                }
                            }
                        });
                    }

                    match_statement.push(quote! {
                        Self::#variant_ident(backtrace) => backtrace.backtrace(),
                    });
                },
                _ => {},
            }
        }
    }

    quote! {
        #[derive(Debug, thiserror::Error)]
        #input
        #(#impl_from)*
        impl #crate_path::Backtraceable for #input_ident {
            fn split(self) -> (String, #crate_path::BT) {
                (self.to_string(), self.backtrace())
            }

            fn backtrace(&self) -> #crate_path::BT {
                match self {
                    #(#match_statement)*
                }
            }
        }
    }
    .into()
}
