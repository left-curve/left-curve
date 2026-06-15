//! Shared types and utilities for the historical indexer.
//!
//! This crate has no dependencies on other `dango-indexer-historical-*` crates and
//! contains items shared between the block source and the projections.

mod block_data;
mod graphql;

pub use {block_data::BlockData, graphql::post_graphql};

/// Convenience alias for `anyhow::Result<T>` used across all historical
/// indexer crates.
pub type AnyResult<T> = anyhow::Result<T>;
