#[cfg(feature = "metrics")]
use crate::{
    graphql::extensions::metrics::{MetricsExtension, init_graphql_metrics},
    metrics::init_core_query_metrics,
};
#[cfg(feature = "tracing")]
use async_graphql::extensions as AsyncGraphqlExtensions;
use {
    crate::{
        context::MinimalContext,
        graphql::{query::core::CoreQuery, telemetry::SentryExtension},
    },
    async_graphql::{EmptyMutation, EmptySubscription, MergedObject, Schema},
};

#[derive(MergedObject, Default)]
#[graphql(name = "Query")] // renamed for backward compatibility
pub struct MinimalQuery(pub CoreQuery);

pub type MinimalSchema = Schema<MinimalQuery, EmptyMutation, EmptySubscription>;

pub fn build_minimal_schema(app_ctx: MinimalContext) -> MinimalSchema {
    #[cfg(feature = "metrics")]
    {
        init_graphql_metrics();
        init_core_query_metrics();
    }

    #[allow(unused_mut)]
    let mut schema_builder =
        Schema::build(MinimalQuery::default(), EmptyMutation, EmptySubscription)
            .extension(SentryExtension);

    #[cfg(feature = "metrics")]
    {
        schema_builder = schema_builder.extension(MetricsExtension);
    }

    #[cfg(feature = "tracing")]
    {
        schema_builder = schema_builder
            .extension(AsyncGraphqlExtensions::Tracing)
            .extension(AsyncGraphqlExtensions::Logger);
    }

    schema_builder
        .data(app_ctx)
        .limit_complexity(300)
        .limit_depth(20)
        .finish()
}
