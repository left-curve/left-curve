use {
    async_graphql::{MergedObject, ObjectType},
    block::BlockQuery,
    indexer_graphql_macro::MyMacro,
};

pub mod block;
pub mod index;

#[derive(MergedObject, Default)]
pub struct Query(BlockQuery);

// #[derive(MergedObject, Default)]
// pub struct Query2(BlockQuery);

// #[MyMacro]
// pub struct Query3(Query, Query2);

// #[derive(MyMacro)]
// pub struct Query3(Query, Query2);

// pub struct Query3(BlockQuery, BlockQuery);

// #[derive(MergedObject, Default)]
// pub struct QueryWithSub<S>(BlockQuery, S)
// where
//     S: ObjectType;

// impl Query {
//    // TODO: merge dango/httpd/src/graphql/query/block.rs
//    pub fn new_with_sub<S>(sub: S) -> Self {
//        Self::default()
//    }
//}

crate::merge_query!(MergedQuery, BlockQuery, BlockQuery);

#[macro_export]
macro_rules! merge_query {
    ($name:ident, $($structs:ty),+ $(,)?) => {
        #[derive(MergedObject, Default)]
        pub struct $name(
            $(
                $structs
            ),*
        );
    };
}

// #[cfg(test)]
// mod test {
//     use super::*;

//     merge_query!(MergedQuery, BlockQuery, BlockQuery);
// }
