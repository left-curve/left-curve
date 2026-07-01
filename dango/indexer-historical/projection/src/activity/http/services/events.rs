//! The `/events` and `/contract-events` read scopes, routed by `#[get]`
//! attribute macros.
//!
//! - `GET /events` — events filtered by `type` (a comma-separated list) and/or
//!   `involved` (an address); **at least one** is required, since an unfiltered
//!   feed has no index anchor (it would be a full-table scan + sort → 400).
//! - `GET /contract-events/{contract}` — a contract's events, optionally
//!   narrowed by `user` (a participant address) and `names` (a comma-separated
//!   list). The mandatory `contract` keeps every reachable combination
//!   index-anchored.
//!
//! Each handler parses its arguments (a malformed address / type / cursor is a
//! 400), runs the matching feed in [`feeds`](super::super::feeds), hydrates the
//! page's `data` from the shared block source ([`hydrate`](super::super::hydrate)),
//! and answers with the JSON page. The shared read handles come from actix app
//! data (`web::Data`), injected by the httpd.

use {
    super::super::{feeds, hydrate},
    crate::activity::event_type::EventType,
    actix_web::{HttpResponse, Scope, get, web},
    dango_indexer_historical_block_source::BlockSource,
    dango_indexer_historical_httpd::ApiError,
    dango_primitives::Addr,
    sea_orm::DatabaseConnection,
    serde::Deserialize,
    std::sync::Arc,
};

/// The `/events` and `/contract-events` scopes.
pub(crate) fn services() -> Vec<Scope> {
    vec![
        web::scope("/events").service(events),
        web::scope("/contract-events").service(contract_events),
    ]
}

/// `/events` arguments — `type` (comma-separated list) and/or `involved`.
#[derive(Deserialize)]
struct EventsQuery {
    #[serde(rename = "type")]
    ty: Option<String>,
    involved: Option<Addr>,
    first: Option<i32>,
    after: Option<String>,
}

/// `/contract-events/{contract}` arguments — optional `user` and `names`
/// (comma-separated list).
#[derive(Deserialize)]
struct ContractEventsQuery {
    user: Option<Addr>,
    names: Option<String>,
    first: Option<i32>,
    after: Option<String>,
}

#[get("")]
async fn events(
    db: web::Data<DatabaseConnection>,
    source: web::Data<Arc<dyn BlockSource>>,
    query: web::Query<EventsQuery>,
) -> Result<HttpResponse, ApiError> {
    let q = query.into_inner();
    let types = parse_types(q.ty)?;

    let mut page = match q.involved {
        // Address-anchored: the optional type set is a residual filter.
        Some(address) => feeds::events_involving(&db, address, types, q.first, q.after).await?,
        // No address anchor — the type set must anchor the query, so it is
        // required (an unfiltered feed would be a full-table scan + sort).
        None => {
            if types.is_empty() {
                return Err(ApiError::bad_request(
                    "`/events` requires at least one of `type` or `involved`",
                ));
            }
            feeds::events_by_type(&db, types, q.first, q.after).await?
        },
    };
    hydrate::hydrate_events(source.get_ref(), &mut page.items).await?;
    Ok(HttpResponse::Ok().json(page))
}

#[get("/{contract}")]
async fn contract_events(
    db: web::Data<DatabaseConnection>,
    source: web::Data<Arc<dyn BlockSource>>,
    contract: web::Path<Addr>,
    query: web::Query<ContractEventsQuery>,
) -> Result<HttpResponse, ApiError> {
    let contract = contract.into_inner();
    let q = query.into_inner();
    let names = parse_names(q.names);

    let mut page = match q.user {
        Some(address) => {
            feeds::contract_events_involving(&db, address, contract, names, q.first, q.after)
                .await?
        },
        None => feeds::contract_events(&db, contract, names, q.first, q.after).await?,
    };
    hydrate::hydrate_events(source.get_ref(), &mut page.items).await?;
    Ok(HttpResponse::Ok().json(page))
}

// ---- argument parsing ----

/// Parse a comma-separated `type` argument into a **set** of [`EventType`]s; an
/// absent or empty argument yields no types. An unknown value is a 400.
///
/// Duplicates are dropped (first-seen order kept): a repeated type would
/// double-count an event in `events_by_type`'s `UNION ALL`, and a type set is
/// idempotent anyway.
fn parse_types(raw: Option<String>) -> Result<Vec<EventType>, ApiError> {
    let Some(list) = raw else {
        return Ok(Vec::new());
    };
    let mut types = list
        .split(',')
        .filter(|name| !name.is_empty())
        .map(parse_event_type)
        .collect::<Result<Vec<_>, _>>()?;

    let mut seen = std::collections::HashSet::new();
    types.retain(|ty| seen.insert(*ty));
    Ok(types)
}

/// Parse one [`EventType`] from its snake_case spelling (`transfer`,
/// `contract_event`, …); an unknown value is a 400.
fn parse_event_type(text: &str) -> Result<EventType, ApiError> {
    use serde::de::{
        IntoDeserializer,
        value::{Error as DeError, StrDeserializer},
    };
    let de: StrDeserializer<DeError> = text.into_deserializer();
    EventType::deserialize(de)
        .map_err(|_: DeError| ApiError::bad_request(format!("unknown event type: {text}")))
}

/// Split a comma-separated `names` argument into a list, dropping empties; an
/// absent argument means "no name filter".
fn parse_names(raw: Option<String>) -> Option<Vec<String>> {
    raw.map(|list| {
        list.split(',')
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .collect()
    })
}
