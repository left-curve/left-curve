//! Historical indexer app orchestration.
//!
//! Wires a single [`BlockSource`](dango_indexer_historical_block_source::BlockSource),
//! the shared [`Committer`](dango_indexer_historical_projection::Committer), and a
//! fixed set of [`Projection`](dango_indexer_historical_projection::Projection)s,
//! drives them in cooperating tasks, and surfaces failures.

mod app;
mod committer;
mod metrics;
mod projection_loop;

pub use {
    crate::metrics::init_metrics, app::App, committer::PgChCommitter,
    projection_loop::projection_loop,
};
