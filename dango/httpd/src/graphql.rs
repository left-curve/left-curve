use {
    async_graphql::{Schema, extensions},
    indexer_httpd::{context::Context, graphql::mutation::Mutation},
    query::Query,
    subscription::Subscription,
};

pub mod query;
pub mod subscription;
pub mod types;

pub(crate) type AppSchema = Schema<Query, Mutation, Subscription>;

pub fn build_schema(app_ctx: Context) -> AppSchema {
    Schema::build(
        Query::default(),
        Mutation::default(),
        Subscription::default(),
    )
    .extension(extensions::Logger)
    .data(app_ctx)
    .finish()
}
