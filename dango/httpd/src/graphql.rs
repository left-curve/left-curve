use {
    async_graphql::{extensions, EmptyMutation, Schema},
    indexer_httpd::context::Context,
    subscription::Subscription,
};

pub mod query;
pub mod subscription;
pub mod types;

pub(crate) type AppSchema = Schema<query::Query, EmptyMutation, Subscription>;

pub fn build_schema(app_ctx: Context) -> AppSchema {
    Schema::build(
        query::Query::default(),
        EmptyMutation,
        Subscription::default(),
    )
    .extension(extensions::Logger)
    .data(app_ctx)
    .finish()
}
