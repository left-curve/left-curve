use {
    proc_macro::TokenStream,
    quote::{format_ident, quote},
    syn::{
        Data, DeriveInput, Field, Fields, FieldsUnnamed, Ident, parse_macro_input, parse_quote,
        punctuated::Punctuated, token::Paren,
    },
};

#[proc_macro_attribute]
pub fn backtrace(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);

    let input_ident = &input.ident;

    let mut impl_from = vec![];
    let mut match_statement = vec![];
    let mut builder_impl = vec![];

    if let Data::Enum(en) = &mut input.data {
        for variant in en.variants.iter_mut() {
            let mut is_fresh = false;
            let mut is_private = false;

            variant.attrs.retain(|a| {
                if a.path().is_ident("backtrace") {
                    let inner = a.parse_args::<Ident>().unwrap();

                    if inner == "new" {
                        is_fresh = true;
                    } else if inner == "private_constructor" {
                        is_private = true;
                    } else {
                        panic!("expected `new` | `private_constructor` attribute, got `{inner}`",);
                    }
                    false
                } else {
                    true
                }
            });

            let pub_ident = if is_private {
                quote! { fn }
            } else {
                quote! { pub fn }
            };

            let variant_ident = &variant.ident;

            match &mut variant.fields {
                Fields::Named(fields) => {
                    match_statement.push(quote! {
                        Self::#variant_ident{backtrace,..} => backtrace.clone(),
                    });

                    let fn_name = to_snake_case(&variant.ident, is_private);

                    let mut inputs = vec![];
                    let mut values = vec![];

                    for f in &fields.named {
                        let ident = f.ident.clone().unwrap();
                        let ty = f.ty.clone();
                        inputs.push(quote! {
                            #ident: #ty,
                        });

                        values.push(quote! {
                            #ident: #ident,
                        });
                    }

                    fields.named.push(parse_quote! {
                        backtrace: ::error_backtrace::BT
                    });

                    builder_impl.push(quote! {
                        #pub_ident #fn_name(#(#inputs)*) -> Self {
                            Self::#variant_ident {
                                #(#values)*
                                backtrace: ::error_backtrace::BT::default(),
                            }
                        }
                    });
                },
                Fields::Unnamed(unamed) => {
                    let mut iter = unamed.unnamed.iter_mut();
                    let field = iter.next().expect("no unnamed fields");
                    let original_ty = &field.ty.clone();

                    field.ty = parse_quote! {
                        ::error_backtrace::BacktracedError<#original_ty>
                    };

                    // Impl conversion from original type to the error type
                    // fresh will capture the backtrace now, otherwise we will
                    // use the backtrace from the original type (require original
                    // type to implement `Backtraceable`).
                    if is_fresh {
                        impl_from.push(quote! {
                            impl From<#original_ty> for #input_ident {
                                fn from(t: #original_ty) -> Self {
                                    Self::#variant_ident(::error_backtrace::BacktracedError::new(t))
                                }
                            }
                        });
                    } else {
                        impl_from.push(quote! {
                            impl From<#original_ty> for #input_ident {
                                fn from(t: #original_ty) -> Self {
                                    let bt = ::error_backtrace::Backtraceable::backtrace(&t);
                                    Self::#variant_ident(::error_backtrace::BacktracedError::new_with_bt(t, bt))
                                }
                            }
                        });
                    }

                    match_statement.push(quote! {
                        Self::#variant_ident(backtrace) => backtrace.backtrace(),
                    });

                    let fn_name = to_snake_case(&variant.ident, is_private);

                    builder_impl.push(quote! {
                        #pub_ident #fn_name(self, value: #original_ty) -> Self {
                            Self::#variant_ident(::error_backtrace::BacktracedError::new(value))
                        }
                    });
                },
                Fields::Unit => {
                    let bt_field: Field = parse_quote! {
                       ::error_backtrace::BT
                    };

                    let mut unnamed = Punctuated::new();
                    unnamed.push(bt_field);

                    variant.fields = Fields::Unnamed(FieldsUnnamed {
                        paren_token: Paren::default(),
                        unnamed,
                    });

                    match_statement.push(quote! {
                        Self::#variant_ident(backtrace) => backtrace.clone(),
                    });

                    let fn_name = to_snake_case(&variant.ident, is_private);

                    builder_impl.push(quote! {
                        #pub_ident #fn_name() -> Self {
                        Self::#variant_ident(::error_backtrace::BT::default())
                       }
                    });
                },
            }
        }
    }

    quote! {
        #input

        #(#impl_from)*

        impl ::error_backtrace::Backtraceable for #input_ident {
            fn into_generic_backtraced_error(self) -> ::error_backtrace::BacktracedError<String> {
                ::error_backtrace::BacktracedError::new_with_bt(self.to_string(), self.backtrace())
            }

            fn backtrace(&self) -> ::error_backtrace::BT {
                match self {
                    #(#match_statement)*
                }
            }

            fn error(&self) -> String {
                self.to_string()
            }
        }

        impl #input_ident {
            #(#builder_impl)*
        }

    }
    .into()
}

fn to_snake_case(s: &Ident, is_private: bool) -> Ident {
    let s = s.to_string();
    let mut result = String::with_capacity(s.len());
    let mut prev_lower = false;

    for c in s.chars() {
        if c.is_uppercase() {
            if prev_lower {
                result.push('_');
            }
            for lc in c.to_lowercase() {
                result.push(lc);
            }
            prev_lower = false;
        } else if c.is_alphanumeric() {
            prev_lower = c.is_lowercase();
            result.push(c);
        } else {
            if !result.ends_with('_') && !result.is_empty() {
                result.push('_');
            }
            prev_lower = false;
        }
    }

    let trimmed = result
        .trim_matches('_')
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_");

    if is_private {
        format_ident!("_{}", trimmed)
    } else {
        format_ident!("{}", trimmed)
    }
}
