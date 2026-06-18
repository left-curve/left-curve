//! Validator-side, in-memory, low-latency event streaming for Dango.
//!
//! This crate provides the `perps_events2` GraphQL subscription: an ephemeral,
//! purely in-memory ring of the last `N` blocks of perps-exchange contract
//! events, broadcast live to bots and algo-traders. It runs in-process with the
//! state machine (no validator -> indexer-node hop), which is what makes it the
//! lowest-latency surface for real-time perps data.
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
//!   module docs.
//! - [`Indexer`] — extracts perps-contract events from each block and appends
//!   them to the ring, INLINE and in height order, from `post_indexing`.
//! - [`Context`] — the reader handle the httpd holds; the `perps_events2`
//!   resolver lives in the httpd crate and drives [`RecentStream::subscribe`].
//!
//! # Future direction
//!
//! - A "new blocks" subscription (consumed by the dedicated indexer node) will
//!   be a second [`RecentStream`] instantiation over full blocks — the
//!   primitive is generic for exactly this.
//! - Once block files / Postgres / ClickHouse move to the indexer node, this
//!   crate will REPLACE `HookedIndexer` as the validator's sole, thin indexer.

mod context;
mod indexer;
mod perps_events;
mod recent_stream;

pub use {
    context::Context,
    indexer::{DEFAULT_RING_CAPACITY, Indexer},
    perps_events::{PerpsEvent, PerpsEventBlock, extract_perps_event_block, make_perps_filter},
    recent_stream::{HasHeight, RecentStream, ResyncRequired},
};
