//! Metric names + descriptions for the read-API httpd — end-to-end GraphQL
//! request stats (the per-feed SQL latencies live in the projection crate).
//!
//! See `design/observability.md`. [`init_metrics`] is always present (a no-op
//! without the `metrics` feature) and is called once at boot by the cli.

// The name constants are referenced only from `#[cfg(feature = "metrics")]`
// code, so a metrics-off build sees them as unused — expected, not a defect.
#![cfg_attr(not(feature = "metrics"), allow(dead_code))]

/// GraphQL requests served (counter).
pub(crate) const GRAPHQL_REQUESTS: &str = "indexer_historical_graphql_requests_total";
/// End-to-end GraphQL request execution (histogram, seconds).
pub(crate) const GRAPHQL_REQUEST_DURATION: &str =
    "indexer_historical_graphql_request_duration_seconds";
/// Concurrent in-flight GraphQL requests (gauge).
pub(crate) const GRAPHQL_IN_FLIGHT: &str = "indexer_historical_graphql_in_flight";

/// Register descriptions for this crate's metrics. Idempotent; a no-op unless the
/// `metrics` feature is on. Called once at boot.
pub fn init_metrics() {
    #[cfg(feature = "metrics")]
    {
        use {
            metrics::{describe_counter, describe_gauge, describe_histogram},
            std::sync::Once,
        };

        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            describe_counter!(GRAPHQL_REQUESTS, "GraphQL requests served");
            describe_histogram!(
                GRAPHQL_REQUEST_DURATION,
                "End-to-end GraphQL request, seconds"
            );
            describe_gauge!(GRAPHQL_IN_FLIGHT, "Concurrent in-flight GraphQL requests");
        });
    }
}
