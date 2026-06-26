//! The GraphQL read API: assemble the merged query root and, when enabled,
//! build the schema and the `serve` task the app supervises.
//!
//! The query root is composed **here** — the cli is the only layer that sees
//! every query object (the core `BlockQuery` plus each projection's). httpd
//! stays projection-agnostic: it takes the assembled roots, injects the shared
//! read handles (the Postgres pool, the block source, and a `BlockLoader`
//! `DataLoader` over it), applies the depth / complexity caps, and serves.

use {
    crate::config::HttpdConfig,
    async_graphql::{EmptySubscription, MergedObject},
    dango_indexer_historical_block_source::{BlockQuery, BlockSource},
    dango_indexer_historical_httpd::{HttpdConfig as ServerConfig, build_schema, serve},
    dango_indexer_historical_projection::ActivityQuery,
    futures::future::BoxFuture,
    sea_orm::DatabaseConnection,
    std::sync::Arc,
};

/// The schema's query root: every query object merged into one. async-graphql
/// flattens the tuple's fields into a single root type, so `block(…)` and the
/// activity feeds sit side by side. Add a projection's query object here when
/// it ships.
#[derive(MergedObject, Default)]
struct Query(BlockQuery, ActivityQuery);

/// Build the read-API server task, or `None` when disabled (the indexer then
/// runs ingest-only). The returned future is the boxed task the app supervises
/// alongside ingest; the schema carries the shared Postgres pool and block
/// source as context.
pub fn task(
    cfg: &HttpdConfig,
    db: DatabaseConnection,
    source: Arc<dyn BlockSource>,
) -> Option<BoxFuture<'static, anyhow::Result<()>>> {
    if !cfg.enabled {
        return None;
    }
    let schema = build_schema(Query::default(), EmptySubscription, db, source);
    Some(serve(schema, ServerConfig {
        bind: cfg.bind.clone(),
    }))
}
