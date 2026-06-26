//! HTTP front door for the historical indexer.
//!
//! A deliberately small actix-web service: given a GraphQL schema built by the
//! composition root (from the projections' query / subscription objects), it
//! injects the shared read-side handles and serves it.
//!
//! - [`build_schema`] takes the assembled roots plus the Postgres pool and the
//!   [`BlockSource`](dango_indexer_historical_block_source::BlockSource), and
//!   returns a read-only schema with those handles injected as context data
//!   (resolvers reach them with `ctx.data::<…>()`).
//! - [`serve`] runs that schema as a boxed, supervised task: `POST /graphql`
//!   (+ an in-browser playground on `GET /graphql`) and a `GET /up` probe.
//!
//! The crate is projection-agnostic — it never names a projection; the
//! composition root assembles the schema and hands the [`App`] the [`serve`]
//! task, which it supervises like any other.
//!
//! [`App`]: dango_indexer_historical_app::App

mod config;
mod schema;
mod server;

pub use {config::HttpdConfig, schema::build_schema, server::serve};
