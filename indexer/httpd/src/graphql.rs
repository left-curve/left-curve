use {
    crate::context::Context,
    async_graphql::{extensions, Schema},
};

pub mod client;
pub mod mutation;
pub mod query;
pub mod subscription;
pub mod types;

pub(crate) type AppSchema = Schema<query::Query, mutation::Mutation, subscription::Subscription>;

pub fn build_schema(app_ctx: Context) -> AppSchema {
    Schema::build(
        query::Query::default(),
        mutation::Mutation::default(),
        subscription::Subscription::default(),
    )
    .extension(extensions::Logger)
    .data(app_ctx)
    .finish()
}
