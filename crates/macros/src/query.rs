use {
    core::panic,
    proc_macro::TokenStream,
    quote::{quote, ToTokens},
    syn::{parse_macro_input, Data, DeriveInput, Ident, Type},
};

pub fn process(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let Data::Enum(data) = input.data else {
        panic!("query message must be an enum")
    };

    let mut generated_structs = Vec::new();
    let mut impl_req_to_enum = Vec::new();
    let mut impl_trait_response = Vec::new();
    let mut impl_as_query_msg = Vec::new();

    for variant in data.variants.into_iter() {
        let return_type: Type = variant
            .attrs
            .iter()
            .find(|attr| attr.path().get_ident().unwrap() == "returns")
            .expect("returns attribute missing")
            .parse_args()
            .expect("only one type supported");

        let variant_name = &variant.ident;

        let request = match variant.fields {
            syn::Fields::Named(variant_ty) => {
                let struct_name =
                    Ident::new(&format!("{variant_name}Request"), variant.ident.span());

                let mut fields_struct_definition = Vec::new();
                let mut fields_struct_into = Vec::new();

                for field in variant_ty.named {
                    let field_name = &field.ident;
                    let field_type = &field.ty;

                    fields_struct_definition.push(quote! {
                        pub #field_name: #field_type,
                    });

                    fields_struct_into.push(quote! {
                        #field_name: val.#field_name,
                    });
                }

                // Generate the struct definition
                generated_structs.push(quote! {
                    pub struct #struct_name {
                        #(#fields_struct_definition)*
                    }
                });

                impl_req_to_enum.push(quote! {
                    impl From<#struct_name> for #name {
                        fn from(val: #struct_name) -> Self {
                            Self::#variant_name {
                                #(#fields_struct_into)*
                            }
                        }
                    }
                });

                struct_name.to_token_stream()
            },
            syn::Fields::Unnamed(variant_ty) => {
                let unamed = variant_ty.unnamed.to_token_stream();

                impl_req_to_enum.push(quote! {
                    impl From<#unamed> for #name {
                        fn from(val: #unamed) -> Self {
                            Self::#variant_name(val)
                        }
                    }
                });

                unamed
            },
            syn::Fields::Unit => panic!("query message cannot contain unit variants"),
        };

        impl_trait_response.push(quote! {
            impl ::grug::QueryResponseType for #request {
                type Response = #return_type;
            }
        });

        impl_as_query_msg.push(quote! {
            impl ::grug::AsQueryMsg for #request {
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
