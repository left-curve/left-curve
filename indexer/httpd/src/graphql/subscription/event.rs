#[cfg(feature = "metrics")]
use grug_httpd::metrics::GaugeGuard;
use {
    super::MAX_PAST_BLOCKS,
    async_graphql::{futures_util::stream::Stream, *},
    futures_util::stream::{StreamExt, once},
    grug_types::Addr,
    indexer_sql::entity,
    itertools::Itertools,
    sea_orm::{
        ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, prelude::Expr,
        sea_query::extension::postgres::PgExpr,
    },
    std::{
        collections::VecDeque,
        ops::RangeInclusive,
        str::FromStr,
        sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
        },
    },
};

#[derive(Clone, InputObject)]
struct Filter {
    r#type: Option<String>,
    data: Option<Vec<FilterData>>,
}

#[derive(Clone, InputObject)]
struct FilterData {
    path: VecDeque<String>,
    check_mode: CheckValue,
    value: Vec<serde_json::Value>,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
enum CheckValue {
    Equal,
    Contains,
}

#[derive(Clone)]
struct ParsedFilter {
    r#type: Option<String>,
    data: Option<Vec<ParsedFilterData>>,
}

#[derive(Clone)]
struct ParsedFilterData {
    path: VecDeque<String>,
    value: ParsedCheckValue,
}

#[derive(Clone)]
enum ParsedCheckValue {
    Equal(serde_json::Value),
    Contains(Vec<serde_json::Value>),
}

#[derive(Default)]
pub struct EventSubscription;

fn precompute_query(
    filters_opt: Option<Vec<ParsedFilter>>,
) -> sea_orm::Select<entity::events::Entity> {
    let mut query = entity::events::Entity::find()
        .order_by_asc(entity::events::Column::BlockHeight)
        .order_by_asc(entity::events::Column::EventIdx);

    let Some(filters) = filters_opt else {
        return query;
    };

    // 1) Build an OR across all filters
    let mut or_filter_expr: Option<sea_orm::sea_query::SimpleExpr> = None;

    for filter in filters {
        // 2) For each filter, build an inner AND expression
        let mut filter_expr: Option<sea_orm::sea_query::SimpleExpr> = None;

        // ---- type filter, if present ----
        if let Some(r#type) = filter.r#type.clone() {
            let expr = Expr::col(entity::events::Column::Type).eq(r#type);
            filter_expr = Some(match filter_expr {
                Some(prev) => prev.and(expr),
                None => expr,
            });
        }

        // ---- data filter, if present ----
        if let Some(data_checks) = filter.data.clone() {
            for check in data_checks {
                // derive all values to match
                let to_match: Vec<serde_json::Value> = match check.value {
                    ParsedCheckValue::Equal(v) => vec![v.clone()],
                    ParsedCheckValue::Contains(vs) => vs.clone(),
                };

                // OR across all to_match values for this check
                let mut check_expr: Option<sea_orm::sea_query::SimpleExpr> = None;

                for val in to_match {
                    // build nested JSON { path[0]: { path[1]: ... val } }
                    let mut json_obj = serde_json::Map::new();
                    for (i, key) in check.path.iter().rev().enumerate() {
                        if i == 0 {
                            json_obj.insert(key.clone(), val.clone());
                        } else {
                            let mut tmp = serde_json::Map::new();
                            tmp.insert(key.clone(), serde_json::Value::Object(json_obj));
                            json_obj = tmp;
                        }
                    }

                    // the JSONB containment expression
                    let expr = Expr::col(entity::events::Column::Data)
                        .contains(serde_json::Value::Object(json_obj));

                    check_expr = Some(match check_expr {
                        Some(prev) => prev.or(expr),
                        None => expr,
                    });
                }

                // combine this check’s result into filter_expr (with AND)
                if let Some(ce) = check_expr {
                    filter_expr = Some(match filter_expr {
                        Some(prev) => prev.and(ce),
                        None => ce,
                    });
                }
            }
        }

        // 3) Now combine each `filter_expr` into the `or_filter_expr` (with OR)
        if let Some(fe) = filter_expr {
            or_filter_expr = Some(match or_filter_expr {
                Some(prev) => prev.or(fe),
                None => fe,
            });
        }
    }

    // 4) Finally apply the combined OR filter
    if let Some(final_expr) = or_filter_expr {
        query = query.filter(final_expr);
    }

    query
}

fn parse_filter(filter: Vec<Filter>) -> Result<Vec<ParsedFilter>, async_graphql::Error> {
    filter
        .into_iter()
        .map(|filter| {
            Ok(ParsedFilter {
                r#type: filter.r#type,
                data: filter
                    .data
                    .map(|data| {
                        data.into_iter()
                            .map(|data| {
                                Ok(ParsedFilterData {
                                    path: data.path,
                                    value: match data.check_mode {
                                        CheckValue::Equal => {
                                            let mut i = data.value.into_iter();
                                            if let (Some(value), None) =
                                                (i.next(), i.next())
                                            {
                                                ParsedCheckValue::Equal(value)
                                            } else {
                                                return Err(async_graphql::Error::new(
                                                    "checkMode::EQUAL must have exactly one value",
                                                ));
                                            }
                                        },
                                        CheckValue::Contains => {
                                            if data.value.is_empty() {
                                                return Err(async_graphql::Error::new(
                                                    "checkMode::CONTAINS must have at least one value",
                                                ));
                                            }

                                            ParsedCheckValue::Contains(data.value)
                                        },
                                    },
                                })
                            })
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?,
            })
        })
        .collect::<Result<Vec<_>, async_graphql::Error>>()
}

impl EventSubscription {
    async fn _events<'a>(
        &self,
        ctx: &Context<'a>,
        // This is used to get the older events in case of disconnection
        since_block_height: Option<u64>,
        query: sea_orm::Select<entity::events::Entity>,
    ) -> Result<impl Stream<Item = Vec<entity::events::Model>> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let latest_block_height = entity::blocks::Entity::find()
            .order_by_desc(entity::blocks::Column::BlockHeight)
            .one(&app_ctx.db)
            .await?
            .map(|block| block.block_height)
            .unwrap_or_default();

        let received_block_height = Arc::new(AtomicU64::new(latest_block_height as u64));

        let block_range = match since_block_height {
            Some(block_height) => block_height as i64..=latest_block_height,
            None => latest_block_height..=latest_block_height,
        };

        if block_range.try_len().unwrap_or(0) > MAX_PAST_BLOCKS {
            return Err(async_graphql::Error::new("`since_block_height` is too old"));
        }

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "events",
            "subscription",
        ));

        let stream = app_ctx.pubsub.subscribe().await?;

        Ok(once({
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();
            let _query = query.clone();

            async move { Self::query_events(&app_ctx.db, block_range, _query).await }
        })
        .chain(stream.then(move |block_height| {
            let query = query.clone();

            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            let received_height = received_block_height.clone();

            async move {
                let current_received = received_height.load(Ordering::Acquire);

                if block_height < current_received {
                    return vec![];
                }

                let events = Self::query_events(
                    &app_ctx.db,
                    (current_received + 1) as i64..=block_height as i64,
                    query,
                )
                .await;

                received_height.store(block_height, Ordering::Release);

                events
            }
        }))
        .filter_map(|events| async move {
            if events.is_empty() {
                None
            } else {
                Some(events)
            }
        }))
    }

    async fn _events_by_addresses<'a>(
        &self,
        ctx: &Context<'a>,
        addresses: Vec<Addr>,
        since_block_height: Option<u64>,
    ) -> Result<impl Stream<Item = Vec<entity::events::Model>> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let latest_block_height = entity::blocks::Entity::find()
            .order_by_desc(entity::blocks::Column::BlockHeight)
            .one(&app_ctx.db)
            .await?
            .map(|block| block.block_height)
            .unwrap_or_default() as u64;

        let received_block_height = Arc::new(AtomicU64::new(latest_block_height));

        let block_range: RangeInclusive<u64> = match since_block_height {
            Some(block_height) => block_height..=latest_block_height,
            None => latest_block_height..=latest_block_height,
        };

        if block_range.try_len().unwrap_or(0) > MAX_PAST_BLOCKS {
            return Err(async_graphql::Error::new("`since_block_height` is too old"));
        }

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "events",
            "subscription",
        ));

        let addresses = Arc::new(addresses);
        let stream = app_ctx.pubsub.subscribe().await?;

        Ok(once({
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();
            let _addresses = addresses.clone();

            async move {
                if block_range.end() < block_range.start() {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(block_range = ?block_range, "`block_range` is descending on `once`, returning an empty vector");
                    return vec![];
                }

                app_ctx
                    .event_cache
                    .read_events(block_range, &_addresses)
                    .await
            }
        })
        .chain(stream.then(move |block_height| {
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            let received_height = received_block_height.clone();
            let _addresses = addresses.clone();

            async move {
                let current_received = received_height.load(Ordering::Acquire);

                if block_height < current_received {
                    return vec![];
                }

                let events = app_ctx
                    .event_cache
                    .read_events((current_received + 1)..=block_height, &_addresses)
                    .await;

                received_height.store(block_height, Ordering::Release);

                events
            }
        }))
        .filter_map(|events| async move {
            if events.is_empty() {
                None
            } else {
                Some(events)
            }
        }))
    }

    async fn query_events(
        db: &DatabaseConnection,
        block_range: RangeInclusive<i64>,
        query: sea_orm::Select<entity::events::Entity>,
    ) -> Vec<entity::events::Model> {
        let query = query.filter(entity::events::Column::BlockHeight.is_in(block_range));
        query
            .all(db)
            .await
            .inspect_err(|_e| {
                #[cfg(feature = "tracing")]
                tracing::error!(%_e, "`get_events` error");
            })
            .unwrap_or_default()
    }
}

#[Subscription]
impl EventSubscription {
    async fn events<'a>(
        &self,
        ctx: &Context<'a>,
        // This is used to get the older events in case of disconnection
        since_block_height: Option<u64>,
        filter: Option<Vec<Filter>>,
    ) -> Result<impl Stream<Item = Vec<entity::events::Model>> + 'a> {
        let filter = filter.map(parse_filter).transpose()?;
        let query = precompute_query(filter);

        self._events(ctx, since_block_height, query).await
    }

    async fn event_by_addresses<'a>(
        &self,
        ctx: &Context<'a>,
        addresses: Vec<String>,
        // This is used to get the older events in case of disconnection
        since_block_height: Option<u64>,
    ) -> Result<impl Stream<Item = Vec<entity::events::Model>> + 'a> {
        let addresses = addresses
            .into_iter()
            .map(|a| Addr::from_str(&a))
            .collect::<Result<Vec<_>, _>>()?;

        self._events_by_addresses(ctx, addresses, since_block_height)
            .await
    }
}
