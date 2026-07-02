//! Shared types and utilities for the archive.
//!
//! This crate has no dependencies on other `dango-archive-*` crates and
//! contains items shared between the block source and the projections.

mod block_data;

pub use block_data::{BlockData, BlockDataExt};

/// Convenience alias for `anyhow::Result<T>` used across all archive
/// crates.
pub type AnyResult<T> = anyhow::Result<T>;
