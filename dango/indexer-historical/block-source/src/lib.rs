//! [`BlockSource`] trait and concrete implementations.
//!
//! - [`LocalBlockSource`] — V1 impl, co-located with the dango node.
//! - [`RemoteBlockSource`] — V2 impl, detached host; composes a
//!   [`BlockStore`], a [`LiveSubscriber`], and a [`BlockFetcher`].
//! - [`BlockFetcher`] — bounded backfill abstraction; [`SentinelBlockFetcher`]
//!   pulls from a sentinel node.

mod block_fetcher;
mod block_source;
mod block_store;
mod httpd_client;
mod live_subscriber;

pub use {
    block_fetcher::{BlockFetcher, FetchStream, SentinelBlockFetcher, SentinelFetcherConfig},
    block_source::{BlockSource, LocalBlockSource, RemoteBlockSource, RemoteBlockSourceConfig},
    block_store::{BlockStore, MemoryBlockStore},
    live_subscriber::LiveSubscriber,
};
