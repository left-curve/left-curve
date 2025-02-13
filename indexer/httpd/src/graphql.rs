use {
    crate::context::Context,
    async_graphql::{extensions, EmptyMutation, Schema},
    query::Query,
};

pub mod query;
pub mod subscription;
pub mod types;

pub(crate) type AppSchema = Schema<Query, EmptyMutation, subscription::Subscription>;

pub fn build_schema(app_ctx: Context) -> AppSchema {
    Schema::build(
        Query::default(),
        EmptyMutation,
        subscription::Subscription::default(),
    )
    .extension(extensions::Logger)
    .data(app_ctx)
    .finish()
}
