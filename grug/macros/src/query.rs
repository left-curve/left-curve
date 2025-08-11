use {
    core::panic,
    proc_macro::TokenStream,
    quote::{ToTokens, quote},
    syn::{Data, DeriveInput, Fields, Ident, Type, parse_macro_input},
};

/// Throughout the function, we will use comments to illustrate how it works,
/// based on the following example:
///
/// ```rust ignore
/// #[derive(grug::QueryRequest)]
/// enum QueryMsg {
///     #[returns(String)]
///     Foo { bar: u64 },
///     #[returns(Addr)]
///     Fuzz(u8),
///     #[returns(Hash256)]
///     Buzz,
/// }
/// ```
pub fn process(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // E.g. `QueryMsg`
    let name = input.ident;

    let Data::Enum(data) = input.data else {
        panic!("query message must be an enum")
    };

    let mut generated_structs: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut impl_into_msg = Vec::new();
    let mut impl_query_request = Vec::new();

    // Iterate through the variants of the query message.
    for variant in data.variants {
        // E.g. `Foo`.
        let variant_name = &variant.ident;

        // Name of the query request for this variant, which we will generate.
        // E.g. `QueryFooRequest`.
        let request_name = Ident::new(&format!("Query{variant_name}Request"), variant.ident.span());

        // Return type for this variant specified in the `#[return]` attribute.
        // E.g. for `Foo`, this would be `String`.
        let return_type: Type = variant
            .attrs
            .iter()
            .find(|attr| attr.path().get_ident().unwrap() == "returns")
            .expect("returns attribute missing")
            .parse_args()
            .expect("only one type supported");

        // Iterate through fields in the query message variant.
        match variant.fields {
            Fields::Named(variant_ty) => {
                let mut fields_struct_definition = Vec::new();
                let mut fields_struct_into = Vec::new();

                for field in variant_ty.named {
                    // E.g. `"bar"`
                    let field_name = &field.ident;
                    // E.g. `u64`
                    let field_type = &field.ty;

                    fields_struct_definition.push(quote! {
                        pub #field_name: #field_type,
                    });

                    fields_struct_into.push(quote! {
                        #field_name: val.#field_name,
                    });
                }

                // Generate the query request struct definition, e.g.
                //
                // ```rust
                // pub struct QueryFooRequest {
                //     pub bar: u64,
                // }
                // ```
                generated_structs.push(quote! {
                    #[grug::derive(Serde)]
                    pub struct #request_name {
                        #(#fields_struct_definition)*
                    }
                });

                // E.g.
                //
                // ```rust
                // impl From<QueryFooRequest> for QueryMsg {
                //     fn from(val: QueryFooRequest) -> Self {
                //         Self::Foo {
                //             bar: val.bar,
                //         }
                //    }
                // }
                // ```
                impl_into_msg.push(quote! {
                    impl From<#request_name> for #name {
                        fn from(val: #request_name) -> Self {
                            Self::#variant_name {
                                #(#fields_struct_into)*
                            }
                        }
                    }
                });
            },
            Fields::Unnamed(variant_ty) => {
                let unnamed = variant_ty.unnamed.into_token_stream();

                // E.g.
                //
                // ```rust
                // pub struct QueryFuzzRequest(u8);
                // ```
                generated_structs.push(quote! {
                    #[grug::derive(Serde)]
                    pub struct #request_name(pub #unnamed);
                });

                // E.g.
                //
                // ```rust
                // impl From<QueryFuzzRequest> for QueryMsg {
                //     fn from(val: QueryFuzzRequest) -> Self {
                //         Self::Fuzz(val.0)
                //     }
                // }
                // ```
                impl_into_msg.push(quote! {
                    impl From<#request_name> for #name {
                        fn from(val: #request_name) -> Self {
                            Self::#variant_name(val.0)
                        }
                    }
                });
            },
            Fields::Unit => {
                // E.g.
                //
                // ```rust
                // pub struct QueryBuzzRequest;
                // ```
                generated_structs.push(quote! {
                    #[grug::derive(Serde)]
                    pub struct #request_name;
                });

                // E.g.
                //
                // ```rust
                // impl From<QueryBuzzRequest> for QueryMsg {
                //     fn from(_val: QueryBuzzRequest) -> Self {
                //         Self::Buzz
                //     }
                // }
                // ```
                impl_into_msg.push(quote! {
                    impl From<#request_name> for #name {
                        fn from(_val: #request_name) -> Self {
                            Self::#variant_name
                        }
                    }
                })
            },
        };

        impl_query_request.push(quote! {
            impl ::grug::QueryRequest for #request_name {
                type Message = #name;
                type Response = #return_type;
            }
        });
    }

    quote! {
        #(#generated_structs)*
        #(#impl_into_msg)*
        #(#impl_query_request)*
    }
    .into()
}
