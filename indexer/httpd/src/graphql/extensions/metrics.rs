use {
    async_graphql::{
        Response, ServerResult, Value,
        extensions::{
            Extension, ExtensionContext, ExtensionFactory, NextExecute, NextResolve, ResolveInfo,
        },
    },
    metrics::{counter, describe_counter, describe_gauge, describe_histogram, histogram},
    std::{sync::Arc, time::Instant},
};

pub struct MetricsExtension;

impl ExtensionFactory for MetricsExtension {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(MetricsExtension)
    }
}

#[async_trait::async_trait]
impl Extension for MetricsExtension {
    /// Called at the beginning of query execution
    async fn execute(
        &self,
        ctx: &ExtensionContext<'_>,
        operation_name: Option<&str>,
        next: NextExecute<'_>,
    ) -> Response {
        let start = Instant::now();

        // Execute the query
        let mut res = next.run(ctx, operation_name).await;

        let duration = start.elapsed().as_secs_f64();

        let is_anonymous = operation_name.is_none();
        if is_anonymous {
            res.extensions.insert(
                "warning".to_string(),
                async_graphql::Value::String(
                    "Consider using named operations for better observability and debugging"
                        .to_string(),
                ),
            );
        }

        let operation = operation_name.unwrap_or("anonymous");

        // Record metrics
        counter!(
            "graphql.requests.total",
            "operation_name" => operation.to_string()
        )
        .increment(1);

        histogram!(
            "graphql.request.duration",
            "operation_name" => operation.to_string()
        )
        .record(duration);

        // Check if there are errors
        if !res.errors.is_empty() {
            counter!(
                "graphql.requests.errors",
                "operation_name" => operation.to_string(),
                "error_count" => res.errors.len().to_string()
            )
            .increment(1);
        }

        res
    }

    /// Called for each field resolution
    async fn resolve(
        &self,
        ctx: &ExtensionContext<'_>,
        info: ResolveInfo<'_>,
        next: NextResolve<'_>,
    ) -> ServerResult<Option<Value>> {
        // Skip introspection queries to avoid noise
        if info.parent_type.starts_with("__") || info.path_node.field_name().starts_with("__") {
            return next.run(ctx, info).await;
        }

        let start = Instant::now();

        let field_name = info.path_node.field_name().to_string();
        let parent_type = info.parent_type.to_string();

        let result = next.run(ctx, info).await;

        let duration = start.elapsed().as_secs_f64();

        // Only record metrics for non-trivial fields
        if duration > 0.001 {
            // 1ms
            histogram!(
                "graphql.field.duration",
                "field" => field_name.clone(),
                "parent_type" => parent_type.clone()
            )
            .record(duration);
        }

        if result.is_err() {
            counter!(
                "graphql.field.errors",
                "field" => field_name,
                "parent_type" => parent_type
            )
            .increment(1);
        }

        result
    }
}

pub fn init_graphql_metrics() {
    describe_counter!("graphql.requests.total", "Total GraphQL requests");
    describe_counter!(
        "graphql.requests.errors",
        "Total GraphQL requests with errors"
    );
    describe_counter!("graphql.field.errors", "GraphQL field resolution errors");
    describe_histogram!(
        "graphql.request.duration",
        "GraphQL request duration in seconds"
    );
    describe_histogram!(
        "graphql.field.duration",
        "GraphQL field resolution duration in seconds"
    );
    describe_gauge!(
        "graphql.subscriptions.active",
        "Number of active GraphQL subscriptions"
    );
}
