use metrics::{describe_histogram, gauge};

/// A guard that increments a gauge when created and decrements it when dropped.
pub struct GaugeGuard {
    metric_name: &'static str,
    operation_name: &'static str,
    operation_type: &'static str,
}

impl GaugeGuard {
    pub fn new(
        metric_name: &'static str,
        operation_name: &'static str,
        operation_type: &'static str,
    ) -> Self {
        gauge!(metric_name, "operation_name" => operation_name, "operation_type" => operation_type)
            .increment(1.0);

        Self {
            metric_name,
            operation_name,
            operation_type,
        }
    }
}

impl Drop for GaugeGuard {
    fn drop(&mut self) {
        gauge!(self.metric_name, "operation_name" => self.operation_name, "operation_type" => self.operation_type).decrement(1.0);
    }
}

/// Register the chain-side query histograms emitted by `CoreQuery` resolvers.
///
/// Distinct from `crate::graphql::extensions::metrics::init_graphql_metrics`,
/// which registers async-graphql operation-level histograms. Both are
/// idempotent describe-only calls.
pub fn init_core_query_metrics() {
    describe_histogram!(
        "http.grug.query_app.duration",
        "Dango query_app duration in seconds"
    );
    describe_histogram!(
        "http.grug.query_status.duration",
        "Dango query_status duration in seconds"
    );
    describe_histogram!(
        "http.grug.simulate.duration",
        "Dango simulate duration in seconds"
    );
}
