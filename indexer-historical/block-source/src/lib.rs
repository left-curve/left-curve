//! [`BlockSource`] trait and concrete implementations.
//!
//! - [`LocalBlockSource`] — V1 impl, co-located with the dango node.

mod block_source;
mod httpd_client;
mod local;

pub use {block_source::BlockSource, local::LocalBlockSource};
