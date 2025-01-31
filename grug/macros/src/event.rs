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

    quote::quote! {
        #input

        impl grug::EventName for #input_name {
            const NAME: &'static str = #name;
        }
    }
    .into()
}
