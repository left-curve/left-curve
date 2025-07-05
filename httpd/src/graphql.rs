use {crate::context::Context, async_graphql::Schema};

pub mod query;
pub mod types;

pub(crate) type AppSchema =
    Schema<query::Query, async_graphql::EmptyMutation, async_graphql::EmptySubscription>;

pub fn build_schema(app_ctx: Context) -> AppSchema {
    Schema::build(
        query::Query::default(),
        async_graphql::EmptyMutation,
        async_graphql::EmptySubscription,
    )
    .data(app_ctx)
    .finish()
}
