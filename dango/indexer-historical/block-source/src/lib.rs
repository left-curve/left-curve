//! [`BlockSource`] trait and concrete implementations.
//!
//! - [`LocalBlockSource`] — V1 impl, co-located with the dango node.
//! - [`RemoteBlockSource`] — V2 impl, detached host; composes a
//!   [`BlockStore`], a node `HttpdClient` for the live tail, and a
//!   [`BlockFetcher`].
//! - [`BlockFetcher`] — bounded backfill abstraction; [`SentinelBlockFetcher`]
//!   pulls from a sentinel node.

mod httpd_client;
mod local;
mod remote;
mod source;

pub use {
    httpd_client::HttpdClient,
    local::LocalBlockSource,
    remote::{
        BlockFetcher, BlockRangeClient, BlockStore, FetchStream, GENESIS_HEIGHT, MAX_BLOCK_RANGE,
        MemoryBlockStore, RemoteBlockSource, RemoteBlockSourceConfig, RocksdbBlockStore,
        SentinelBlockFetcher, SentinelFetcherConfig,
    },
    source::BlockSource,
};
