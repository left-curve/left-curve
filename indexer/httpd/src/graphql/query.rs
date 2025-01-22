use {
    async_graphql::{MergedObject, ObjectType},
    block::BlockQuery,
};

pub mod block;
pub mod index;

#[derive(MergedObject, Default)]
pub struct Query(BlockQuery);

// #[derive(MergedObject, Default)]
pub struct Query2(BlockQuery);

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

#[cfg(test)]
mod test {
    use super::*;

    merge_query!(MergedQuery, BlockQuery, BlockQuery);
}
