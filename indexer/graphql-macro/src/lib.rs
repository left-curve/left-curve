use {
    proc_macro::TokenStream,
    quote::quote,
    syn::{parse_macro_input, Data, DeriveInput, Fields, Type},
};

#[proc_macro_attribute]
pub fn MyMacro(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident;

    // Get tuple struct fields
    let fields = match input.data {
        Data::Struct(data) => match data.fields {
            Fields::Unnamed(fields) => fields.unnamed,
            _ => panic!("MyMacro only works on tuple structs"),
        },
        _ => panic!("MyMacro only works on structs"),
    };

    // Get the field types directly from the input tuple struct fields
    let field_types = fields.iter().map(|f| &f.ty).collect::<Vec<_>>();

    // Generate the output struct with the field types
    let expanded = quote! {
        pub struct #struct_name(
            #(#field_types),*
        );
    };

    TokenStream::from(expanded)
}
