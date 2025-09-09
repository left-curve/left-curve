#[cfg(feature = "metrics")]
use crate::metrics::init_graphql_metrics;
use {crate::context::Context, async_graphql::Schema};

pub mod query;
// pub mod subscription;
pub mod types;

pub(crate) type AppSchema =
    Schema<query::Query, async_graphql::EmptyMutation, async_graphql::EmptySubscription>;

pub fn build_schema(app_ctx: Context) -> AppSchema {
    #[cfg(feature = "metrics")]
    init_graphql_metrics();

    Schema::build(
        query::Query::default(),
        async_graphql::EmptyMutation,
        async_graphql::EmptySubscription,
    )
    .data(app_ctx)
    .finish()
}
