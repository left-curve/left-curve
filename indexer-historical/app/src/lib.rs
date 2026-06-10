//! Historical indexer app orchestration.
//!
//! Wires a single [`BlockSource`](indexer_historical_block_source::BlockSource),
//! the shared [`Committer`](indexer_historical_projection::Committer), and a
//! fixed set of [`Projection`](indexer_historical_projection::Projection)s,
//! drives them in cooperating tasks, and surfaces failures.

mod app;
mod committer;
mod projection_loop;

pub use {app::App, committer::PgChCommitter, projection_loop::projection_loop};
