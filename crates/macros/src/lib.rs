mod derive;
mod export;
mod index_list;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn derive(attr: TokenStream, input: TokenStream) -> TokenStream {
    derive::process(attr, input)
}

#[proc_macro_attribute]
pub fn export(_attr: TokenStream, input: TokenStream) -> TokenStream {
    export::process(input)
}

/// Implement the `IndexList` trait for an index struct.
///
/// ## Example
///
/// ```rust ignore
/// #[grug::derive(borsh)]
/// pub struct Test {
///    pub foo: String,
///    pub bar: u64,
/// }
///
/// #[grug::index_list(Test, u64)]
/// pub struct TestIndexes<'a> {
///    pub foo: MultiIndex<'a, u64, String, Test, Borsh>,
///    pub bar: UniqueIndex<'a, u64, Test, Borsh>,
/// }
/// ```
#[proc_macro_attribute]
pub fn index_list(attr: TokenStream, input: TokenStream) -> TokenStream {
    index_list::process(attr, input)
}
