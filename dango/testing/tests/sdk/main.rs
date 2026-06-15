//! Integration tests for the Rust SDK (`dango-sdk`).
//!
//! These exercise the SDK's HTTP and WebSocket clients against a mock
//! dango-httpd backed by `dango-testing`'s in-process chain. They live here,
//! rather than in `dango-sdk`, so that `dango-testing` is the importer — which
//! keeps the test harness a leaf that no shipped crate depends on.

mod utils;

mod client;
mod core;
mod smoke;
