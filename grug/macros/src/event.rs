use {
    proc_macro::TokenStream,
    syn::{
        DeriveInput, LitStr,
        parse::{Parse, ParseStream},
        parse_macro_input,
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

    // Note: Ideally, we do a blanket implementation of
    //
    // ```rust
    // impl TryFrom<E> for ContractEvent
    // where
    //     E: EventName + Serialize,
    // {
    //     // ...
    // }
    // ```
    //
    // However, this doesn't work because it's possible that a type implements
    // both `EventName + Serialize` and `Into<ContractEvent>`, so there're two
    // conflicting implementations of `TryInto<ContractEvent>`.
    //
    // As a workaround, we implement `TryInto<ContractEvent>` individually for
    // each event type instead of a blanket implementation.
    quote::quote! {
        #input

        impl ::grug::EventName for #input_name {
            const EVENT_NAME: &'static str = #name;
        }

        impl TryFrom<#input_name> for ::grug::ContractEvent {
            type Error = ::grug::StdError;

            fn try_from(event: #input_name) -> ::grug::StdResult<Self> {
                Self::new(&event)
            }
        }
    }
    .into()
}
