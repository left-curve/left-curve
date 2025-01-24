use {
    async_graphql::{extensions, EmptyMutation, EmptySubscription, MergedObject, Schema},
    indexer_httpd::{
        context::Context,
        graphql::query::{block::BlockQuery, message::MessageQuery},
    },
    query::transfer::TransferQuery,
};

pub mod query;
pub mod types;

#[derive(MergedObject, Default)]
pub struct Query(BlockQuery, MessageQuery, TransferQuery);

pub(crate) type AppSchema = Schema<Query, EmptyMutation, EmptySubscription>;

pub fn build_schema(app_ctx: Context) -> AppSchema {
    Schema::build(Query::default(), EmptyMutation, EmptySubscription)
        .extension(extensions::Logger)
        .data(app_ctx)
        .finish()
}
