use {
    core::panic,
    proc_macro::TokenStream,
    quote::{quote, ToTokens},
    syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Type},
};

/// Throughout the function, we will use comments to illustrate how it works,
/// based on the following example:
///
/// ```rust
/// #[derive(grug::Query)]
/// enum QueryMsg {
///     #[returns(String)]
///     Foo { bar: u64 },
///     #[returns(Buzz)]
///     Fuzz(i128),
/// }
/// ```
pub fn process(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // E.g. `QueryMsg`
    let name = input.ident;

    let Data::Enum(data) = input.data else {
        panic!("query message must be an enum")
    };

    let mut generated_structs = Vec::new();
    let mut impl_req_to_enum = Vec::new();
    let mut impl_trait_response = Vec::new();
    let mut impl_as_query_msg = Vec::new();

    // Iterate through the variants of the query message.
    for variant in data.variants {
        // E.g. `Foo`.
        let variant_name = &variant.ident;

        // Name of the query request for this variant, which we will generate.
        // E.g. `QueryFooRequest`.
        let request_name = Ident::new(&format!("Query{variant_name}Request"), variant.ident.span());

        // Return type for this variant specified in the `#[return]` attribute.
        // E.g. for `Foo`, this would be `String`; for `Fuzz`, this would be `Buzz`.
        let return_type: Type = variant
            .attrs
            .iter()
            .find(|attr| attr.path().get_ident().unwrap() == "returns")
            .expect("returns attribute missing")
            .parse_args()
            .expect("only one type supported");

        // Iterate through fields in the query message variant.
        // In the example, there is one variant: `bar`.
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
                impl_req_to_enum.push(quote! {
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

                // Generate the query request struct definition, e.g.
                //
                // ```rust
                // pub struct QueryFuzzRequest(i128);
                // ```
                generated_structs.push(quote! {
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
                impl_req_to_enum.push(quote! {
                    impl From<#request_name> for #name {
                        fn from(val: #request_name) -> Self {
                            Self::#variant_name(val.0)
                        }
                    }
                });
            },
            Fields::Unit => panic!("query message cannot contain unit variants"),
        };

        impl_trait_response.push(quote! {
            impl ::grug::QueryResponseType for #request_name {
                type Response = #return_type;
            }
        });

        impl_as_query_msg.push(quote! {
            impl ::grug::AsQueryMsg for #request_name {
                type QueryMsg = #name;
            }
        });
    }

    quote! {
        #(#generated_structs)*
        #(#impl_req_to_enum)*
        #(#impl_trait_response)*
        #(#impl_as_query_msg)*
    }
    .into()
}
