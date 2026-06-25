//! [`BlockSource`] trait and concrete implementations.
//!
//! - [`LocalBlockSource`] — V1 impl, co-located with the dango node.
//! - [`RemoteBlockSource`] — V2 impl, detached host; composes a
//!   [`BlockStore`], a [`LiveSubscriber`], and a [`BlockFetcher`].
//! - [`BlockFetcher`] — bounded backfill abstraction; [`SentinelBlockFetcher`]
//!   pulls from a sentinel node.

mod httpd_client;
mod local;
mod remote;
mod source;

pub use {
    local::LocalBlockSource,
    remote::{
        BlockFetcher, BlockRangeClient, BlockStore, FetchStream, FullBlockSubscriber,
        GENESIS_HEIGHT, LiveSubscriber, MAX_BLOCK_RANGE, MemoryBlockStore, RemoteBlockSource,
        RemoteBlockSourceConfig, RocksdbBlockStore, SentinelBlockFetcher, SentinelFetcherConfig,
        SentinelRangeClient,
    },
    source::BlockSource,
};
