use {
    core::panic,
    proc_macro::TokenStream,
    proc_macro2::Span,
    quote::{quote, ToTokens},
    syn::{parse_macro_input, Data, DeriveInput, Fields, Ident},
};

pub fn process(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let Data::Enum(data) = &input.data else {
        panic!("query message must be an enum")
    };

    let enum_name = &input.ident;

    let impls = data
        .variants
        .iter()
        .map(|variant| {
            let Fields::Unnamed(inner) = &variant.fields else {
                panic!("query message variant must be unnamed")
            };

            let inner = inner.unnamed.clone().into_token_stream();

            let variant_name = &variant.ident;
            let variant_name_str = variant_name.to_string();

            let fn_name = Ident::new(
                &format!("as_{}", to_snake_case(&variant_name_str)),
                Span::call_site(),
            );

            let enum_name_str = enum_name.to_string();

            quote! {
                pub fn #fn_name(self) -> #inner {
                    let Self::#variant_name(inner) = self else {
                        panic!("{} is not {}", #variant_name_str, #enum_name_str);
                    };
                    inner
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        #input
        impl #enum_name {
            #(#impls)*
        }
    }
    .into()
}

fn to_snake_case(s: &str) -> String {
    let mut snake_case = String::new();

    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i != 0 {
                snake_case.push('_');
            }
            snake_case.push(ch.to_ascii_lowercase());
        } else if ch.is_whitespace() {
            snake_case.push('_');
        } else {
            snake_case.push(ch);
        }
    }

    snake_case
}
