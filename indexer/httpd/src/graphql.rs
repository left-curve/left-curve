use {
    crate::context::Context,
    async_graphql::{extensions, EmptyMutation, EmptySubscription, Schema},
    query::Query,
};

pub mod query;
pub mod types;

pub(crate) type AppSchema = Schema<Query, EmptyMutation, EmptySubscription>;

pub fn build_schema(app_ctx: Context) -> AppSchema {
    Schema::build(Query::default(), EmptyMutation, EmptySubscription)
        .extension(extensions::Logger)
        .data(app_ctx)
        .finish()
}
