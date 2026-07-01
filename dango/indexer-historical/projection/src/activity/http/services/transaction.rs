//! The `/transactions` read scope — the two transaction feeds, routed by
//! `#[get]` attribute macros.
//!
//! Each handler parses its path / query arguments (a malformed hash, address or
//! cursor is a 400), runs the matching feed in [`feeds`](super::super::feeds)
//! against the shared Postgres pool, hydrates the page's `tx` / `outcome` from
//! the shared block source ([`hydrate`](super::super::hydrate)), and answers with
//! the JSON page. The shared read handles come from actix app data (`web::Data`),
//! injected by the httpd.

use {
    super::super::{
        feeds, hydrate,
        types::{AddressRole, UnitKind},
    },
    actix_web::{HttpResponse, Scope, get, web},
    dango_indexer_historical_block_source::BlockSource,
    dango_indexer_historical_httpd::ApiError,
    dango_primitives::{Addr, Hash256},
    sea_orm::DatabaseConnection,
    serde::Deserialize,
    std::sync::Arc,
};

/// The `/transactions` scope: the by-hash lookup and the "involving" feed.
pub(crate) fn services() -> Scope {
    web::scope("/transactions")
        .service(by_hash)
        .service(involving)
}

/// `transactionsInvolving` arguments.
#[derive(Deserialize)]
struct InvolvingQuery {
    role: Option<AddressRole>,
    kind: Option<UnitKind>,
    first: Option<i32>,
    after: Option<String>,
}

#[get("/by-hash/{hash}")]
async fn by_hash(
    db: web::Data<DatabaseConnection>,
    source: web::Data<Arc<dyn BlockSource>>,
    hash: web::Path<Hash256>,
) -> Result<HttpResponse, ApiError> {
    let mut items = feeds::transactions_by_hash(&db, hash.into_inner()).await?;
    hydrate::hydrate_transactions(source.get_ref(), &mut items).await?;
    Ok(HttpResponse::Ok().json(items))
}

#[get("/involving/{address}")]
async fn involving(
    db: web::Data<DatabaseConnection>,
    source: web::Data<Arc<dyn BlockSource>>,
    address: web::Path<Addr>,
    query: web::Query<InvolvingQuery>,
) -> Result<HttpResponse, ApiError> {
    let q = query.into_inner();
    let mut page =
        feeds::transactions_involving(&db, address.into_inner(), q.role, q.kind, q.first, q.after)
            .await?;
    hydrate::hydrate_transactions(source.get_ref(), &mut page.items).await?;
    Ok(HttpResponse::Ok().json(page))
}
