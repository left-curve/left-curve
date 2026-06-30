//! Validator-side, in-memory, low-latency event streaming for Dango.
//!
//! This crate provides two ephemeral, purely in-memory real-time feeds, each a
//! ring of the last `N` blocks broadcast live, in-process with the state
//! machine (no validator -> indexer-node hop) — the lowest-latency surface for
//! real-time data, served to clients over the WebSocket `/ws` endpoint as the
//! `perpsEvents` and `fullBlock` channels:
//!
//! - per-block perps-exchange contract events, for bots and algo-traders.
//! - each finalized block in full (`Block` + `BlockOutcome`).
//!
//! It implements [`dango_app::Indexer`] — but, despite the name, it does no
//! durable indexing. It only maintains in-memory state for real-time
//! subscriptions. It is wired into `HookedIndexer` as one more field for now.
//!
//! # Architecture
//!
//! - [`RecentStream`] — the generic in-memory ring + live broadcast, with a
//!   reliable subscription builder (snapshot then live, in strict height order,
//!   no silent drops). It fixes the `event_by_addresses` failure modes; see its
//!   module docs. It is instantiated twice: over [`PerpsEventBlock`] and over
//!   [`BlockAndOutcome`].
//! - [`Indexer`] — stashes each block at `index_block` (FinalizeBlock) and
//!   publishes both rings from `post_indexing`, in height order, once the
//!   block is committed (the perps address it also needs only arrives with
//!   `app_cfg` there).
//! - [`Context`] — the reader handle the httpd holds; the `/ws` WebSocket
//!   handler lives in the httpd crate and drives [`RecentStream::subscribe`].
//!
//! # Future direction
//!
//! - Once block files / Postgres / ClickHouse move to the dedicated indexer
//!   node, this crate will REPLACE `HookedIndexer` as the validator's sole, thin
//!   indexer; the full-block ring is what that node consumes to stay in sync.

mod context;
mod indexer;
mod perps_events;
mod recent_stream;

pub use {context::*, indexer::*, perps_events::*, recent_stream::*};
