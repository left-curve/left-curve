//! Historical indexer app orchestration.
//!
//! Wires a single [`BlockSource`](indexer_historical_block_source::BlockSource)
//! and a fixed set of
//! [`Projection`](indexer_historical_projection::Projection)s, drives them in
//! cooperating tasks, and surfaces failures.

mod app;
mod projection_loop;

pub use {app::App, projection_loop::projection_loop};
