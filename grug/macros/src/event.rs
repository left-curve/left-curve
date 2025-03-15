use {
    proc_macro::TokenStream,
    syn::{
        parse::{Parse, ParseStream},
        parse_macro_input, DeriveInput, LitStr,
    },
};

struct Args(String);

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse::<LitStr>()?;

        if !input.is_empty() {
            return Err(input.error("expected only a single identifier"));
        }

        Ok(Self(name.value()))
    }
}

pub fn process(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attr as Args);
    let input = parse_macro_input!(input as DeriveInput);

    let name = attrs.0;
    let input_name = input.ident.clone();

    // We implement `TryInto<ContractEvent>` for both `T` and `&T`, such that
    // when calling `Response::add_event`, there is flexibility whether to pass
    // by reference or by value, at the developer's choice.
    quote::quote! {
        #input

        impl TryFrom<&#input_name> for ::grug::ContractEvent {
            type Error = ::grug::StdError;

            fn try_from(value: &#input_name) -> Result<Self, Self::Error> {
                ::grug::ContractEvent::new(#name, value)
            }
        }

        impl TryFrom<#input_name> for ::grug::ContractEvent {
            type Error = ::grug::StdError;

            fn try_from(value: #input_name) -> Result<Self, Self::Error> {
                ::grug::ContractEvent::new(#name, &value)
            }
        }
    }
    .into()
}
