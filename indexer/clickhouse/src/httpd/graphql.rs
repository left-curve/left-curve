#[cfg(feature = "metrics")]
use indexer_httpd::graphql::extensions::metrics::{MetricsExtension, init_graphql_metrics};
use {
    crate::context::Context,
    async_graphql::{
        EmptyMutation, EmptySubscription, Schema,
        extensions::{self as AsyncGraphqlExtensions},
    },
    indexer_httpd::graphql::telemetry::SentryExtension,
};

pub mod query;
pub mod subscription;

pub(crate) type AppSchema = Schema<query::Query, EmptyMutation, EmptySubscription>;

pub fn build_schema(app_ctx: Context) -> AppSchema {
    #[cfg(feature = "metrics")]
    init_graphql_metrics();

    #[allow(unused_mut)]
    let mut schema_builder = Schema::build(
        query::Query::default(),
        EmptyMutation,
        EmptySubscription,
    )
    .extension(AsyncGraphqlExtensions::Logger)
    // .extension(AsyncGraphqlExtensions::Tracing)
    .extension(SentryExtension);

    #[cfg(feature = "metrics")]
    {
        schema_builder = schema_builder.extension(MetricsExtension);
    }

    schema_builder
        .data(app_ctx)
        .limit_complexity(300)
        .limit_depth(20)
        .finish()
}
