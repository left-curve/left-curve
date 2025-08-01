mod backtrace;
mod derive;
mod event;
mod export;
mod index_list;
mod query;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn event(attr: TokenStream, input: TokenStream) -> TokenStream {
    event::process(attr, input)
}

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
/// The macro takes exactly two attributes: the indexed map's primary key and
/// data type, respectively. Each can be a tuple of arbitrarily many elements.
///
/// ## Example
///
/// The following creates an `IndexedMap` with the primary key type `u32` and
/// data type `Test`.
///
/// ```rust ignore
/// use grug::{IndexedMap, MultiIndex, UniqueIndex};
///
/// #[grug::derive(borsh)]
/// pub struct Test {
///     pub foo: String,
///     pub bar: u64,
/// }
///
/// #[grug::index_list(u32, Test)]
/// pub struct TestIndexes<'a> {
///     pub foo: MultiIndex<'a, u32, String, Test>,
///     pub bar: UniqueIndex<'a, u32, u64, Test>,
/// }
///
/// const TEST: IndexedMap<u64, String, TestIndexes> = IndexedMap::new("test", TestIndexes {
///     foo: MultiIndex::new(|_, test| test.foo.clone(), "test", "test__foo"),
///     bar: UniqueIndex::new(|_, test| test.bar, "test", "test__bar"),
/// });
/// ```
#[proc_macro_attribute]
pub fn index_list(attr: TokenStream, input: TokenStream) -> TokenStream {
    index_list::process(attr, input)
}

#[proc_macro_derive(QueryRequest, attributes(returns))]
pub fn derive_query(input: TokenStream) -> TokenStream {
    query::process(input)
}

#[proc_macro_attribute]
pub fn backtrace(attr: TokenStream, input: TokenStream) -> TokenStream {
    backtrace::process(attr, input)
}

#[proc_macro_attribute]
pub fn backtrace_variant(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // non fa nulla, è solo per permettere sintassi come #[backtrace_variant(fresh)]
    item
}
