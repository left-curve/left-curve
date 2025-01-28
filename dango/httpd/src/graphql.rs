use {
    async_graphql::{extensions, EmptyMutation, EmptySubscription, Schema},
    indexer_httpd::context::Context,
};

pub mod query;
pub mod types;

pub(crate) type AppSchema = Schema<query::Query, EmptyMutation, EmptySubscription>;

pub fn build_schema(app_ctx: Context) -> AppSchema {
    Schema::build(query::Query::default(), EmptyMutation, EmptySubscription)
        .extension(extensions::Logger)
        .data(app_ctx)
        .finish()
}
