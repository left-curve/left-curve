//! The activity projection's read **scopes**, one module per resource —
//! mirroring the in-process indexer's `routes/` layout (`#[get]`-routed handlers
//! grouped in a `web::scope`).
//!
//! - [`transaction`] — the `/transactions` scope;
//! - [`events`] — the `/events` and `/contract-events` scopes.
//!
//! [`scopes`] gathers them into the list the app mounts on the shared httpd. The
//! handlers reach the shared Postgres pool and block source through actix app
//! data, so a scope carries no state and is cheap to rebuild per worker.

mod events;
mod transaction;

use actix_web::Scope;

/// Every read scope the activity projection exposes — `/transactions`,
/// `/events`, and `/contract-events`.
pub(crate) fn scopes() -> Vec<Scope> {
    let mut scopes = vec![transaction::services()];
    scopes.extend(events::services());
    scopes
}

#[cfg(test)]
mod tests {
    use {
        super::scopes,
        actix_web::{App, test, web},
        dango_indexer_historical_block_source::BlockSource,
        dango_indexer_historical_httpd::ApiError,
        dango_indexer_historical_types::{AnyResult, BlockData},
        sea_orm::Database,
        std::sync::Arc,
    };

    /// A `BlockSource` the rejection paths never call (they fail before
    /// hydration): the read handles only need to *exist* as app data so actix
    /// extraction reaches the handler.
    struct StubSource;

    #[async_trait::async_trait]
    impl BlockSource for StubSource {
        async fn run(self: Arc<Self>) -> AnyResult<()> {
            Ok(())
        }

        async fn get(&self, _height: u64) -> AnyResult<Option<BlockData>> {
            Ok(None)
        }

        fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Arc<BlockData>> {
            tokio::sync::broadcast::channel(1).1
        }

        async fn contiguous_frontier(&self) -> AnyResult<Option<u64>> {
            Ok(None)
        }
    }

    /// Every malformed or unanchored request is rejected with a **400** before
    /// any database query — exercising actix's path / query extraction and our
    /// own guardrails (the unknown-type / unanchored-`/events` / bad-cursor 400s
    /// short-circuit before the Postgres SQL runs, so a throwaway SQLite handle
    /// that is never queried suffices). The happy paths, which need real
    /// Postgres, are covered by the `backfill` integration test.
    #[actix_web::test]
    async fn rejects_malformed_or_unanchored_requests() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("connect the throwaway db");
        let source: Arc<dyn BlockSource> = Arc::new(StubSource);

        // Mirror the read-API app the httpd assembles: the shared read handles
        // plus the path / query error handlers that make a malformed argument a
        // uniform 400 (without `PathConfig`, actix would 404 a bad path param).
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(db))
                .app_data(web::Data::new(source))
                .app_data(web::PathConfig::default().error_handler(|err, _req| {
                    ApiError::bad_request(format!("invalid path parameter: {err}")).into()
                }))
                .app_data(web::QueryConfig::default().error_handler(|err, _req| {
                    ApiError::bad_request(format!("invalid query parameter: {err}")).into()
                }))
                .configure(|cfg| {
                    for scope in scopes() {
                        cfg.service(scope);
                    }
                }),
        )
        .await;

        for path in [
            "/events",                                // no anchor: neither type nor involved
            "/events?type=not_a_type",                // unknown event type
            "/events?type=transfer&after=zz",         // malformed cursor (not hex)
            "/events?involved=not_an_address",        // malformed address (query)
            "/contract-events/not_an_address",        // malformed address (path)
            "/transactions/by-hash/not_a_hash",       // malformed hash (path)
            "/transactions/involving/not_an_address", // malformed address (path)
        ] {
            let req = test::TestRequest::get().uri(path).to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status().as_u16(),
                400,
                "GET {path} should be a 400, got {}",
                resp.status()
            );
        }
    }
}
