#[cfg(feature = "metrics")]
use grug_httpd::metrics::GaugeGuard;
use {
    super::MAX_PAST_BLOCKS,
    async_graphql::{futures_util::stream::Stream, *},
    futures_util::stream::{StreamExt, once},
    indexer_sql::entity,
    itertools::Itertools,
    sea_orm::{
        ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, prelude::Expr,
        sea_query::extension::postgres::PgExpr,
    },
    std::{
        collections::VecDeque,
        ops::RangeInclusive,
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
    async fn get_events(
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

        let filter = filter.map(parse_filter).transpose()?;

        let query = precompute_query(filter);

        let once_query = query.clone();

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "events",
            "subscription",
        ));

        Ok(once({
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            async move { Self::get_events(&app_ctx.db, block_range, once_query).await }
        })
        .chain(app_ctx.pubsub.subscribe().await?.then(move |block_height| {
            let query = query.clone();

            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            let received_height = received_block_height.clone();

            async move {
                let current_received = received_height.load(Ordering::Acquire);

                if block_height < current_received {
                    return vec![];
                }

                let events = Self::get_events(
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
}

#[cfg(test)]
mod tests {
    use {
        super::*, chrono::NaiveDateTime, sea_orm::ActiveValue::Set, serde_json::json, uuid::Uuid,
    };

    mod db_utils {
        use {
            indexer_sql_migration::MigratorTrait,
            pg_embed::{
                pg_enums::PgAuthMethod,
                pg_fetch::PgFetchSettings,
                postgres::{PgEmbed, PgSettings},
            },
            sea_orm::{Database, DatabaseConnection},
            std::{net::TcpListener, ops::Deref},
        };

        pub async fn migrate_db<M: MigratorTrait>(
            db_connection: &DatabaseConnection,
        ) -> anyhow::Result<()> {
            println!("🔄 Starting migration with URL: {db_connection:?}");
            M::up(db_connection, None).await?;
            println!("✅ Migration completed!");
            Ok(())
        }

        pub async fn create_db() -> anyhow::Result<PgInstance> {
            let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
            let port = listener.local_addr().unwrap().port();
            drop(listener);

            let mut pg = PgEmbed::new(
                PgSettings {
                    database_dir: format!("./pg_data_{port}").into(),
                    port,
                    user: "postgres".into(),
                    password: "postgres".into(),
                    persistent: false,
                    timeout: None,
                    migration_dir: None,
                    auth_method: PgAuthMethod::Plain,
                },
                PgFetchSettings {
                    version: pg_embed::pg_fetch::PG_V15,
                    ..Default::default()
                },
            )
            .await?;

            pg.setup().await?;

            pg.start_db().await?;

            let db_name = format!("test_db_{port}");

            pg.create_database(&db_name).await?;

            let uri = pg.full_db_uri(&db_name);

            Ok(PgInstance {
                connection: Database::connect(&uri).await?,
                _pg: pg,
            })
        }

        pub struct PgInstance {
            connection: DatabaseConnection,
            _pg: PgEmbed,
        }

        impl PgInstance {
            pub fn db_connection(&self) -> &DatabaseConnection {
                &self.connection
            }
        }

        impl Deref for PgInstance {
            type Target = DatabaseConnection;

            fn deref(&self) -> &Self::Target {
                &self.connection
            }
        }
    }

    fn filter<const A: usize>(
        r#type: Option<&str>,
        data: [(&[&str], ParsedCheckValue); A],
    ) -> ParsedFilter {
        ParsedFilter {
            r#type: r#type.map(|s| s.to_string()),
            data: Some(
                data.iter()
                    .map(|(path, value)| ParsedFilterData {
                        path: path.iter().map(|s| s.to_string()).collect(),
                        value: value.clone(),
                    })
                    .collect(),
            ),
        }
    }

    fn filters<const A: usize>(
        filters: [ParsedFilter; A],
    ) -> sea_orm::Select<entity::events::Entity> {
        precompute_query(Some(filters.to_vec()))
    }

    #[tokio::test]
    #[ignore = "not working in CI"]
    async fn events_filter() {
        let db = db_utils::create_db().await.unwrap();

        db_utils::migrate_db::<indexer_sql_migration::Migrator>(db.db_connection())
            .await
            .unwrap();

        let entities = [
            (
                "contract_event",
                json!({
                    "type": "order_filled",
                    "data": {
                      "user": "rhaki"
                    }
                }),
            ),
            (
                "contract_event",
                json!({
                    "type": "order_filled",
                    "data": {
                      "user": "larry"
                    }
                }),
            ),
            (
                "contract_event",
                json!({
                    "type": "order_created",
                    "data": {
                      "user": "foo"
                    }
                }),
            ),
            (
                "contract_event",
                json!({
                    "type": "order_created",
                    "data": {
                      "user": "rhaki"
                    }
                }),
            ),
            (
                "transfer",
                json!({
                    "to": "rhaki",
                    "coins": {
                      "usdc": "1000",
                      "btc": "200"
                    }
                }),
            ),
        ]
        .into_iter()
        .map(|(r#type, data)| entity::events::ActiveModel {
            id: Set(Uuid::new_v4()),
            parent_id: Set(None),
            transaction_id: Set(None),
            message_id: Set(None),
            created_at: Set(NaiveDateTime::default()),
            r#type: Set(r#type.to_string()),
            method: Set(None),
            event_status: Set(entity::events::EventStatus::Ok),
            commitment_status: Set(grug_types::FlatCommitmentStatus::Committed),
            transaction_type: Set(0),
            transaction_idx: Set(0),
            message_idx: Set(None),
            event_idx: Set(0),
            data: Set(data),
            block_height: Set(1),
        })
        .collect::<Vec<_>>();

        entity::events::Entity::insert_many(entities)
            .exec(db.db_connection())
            .await
            .unwrap();

        let f = filters([filter(Some("contract_event"), [(
            &["type"],
            ParsedCheckValue::Equal(json!("order_filled")),
        )])]);
        let events = EventSubscription::get_events(db.db_connection(), 1..=1, f).await;
        assert_eq!(events.len(), 2);

        let f = filters([filter(Some("contract_event"), [(
            &["type"],
            ParsedCheckValue::Equal(json!("order_created")),
        )])]);
        let events = EventSubscription::get_events(db.db_connection(), 1..=1, f).await;
        assert_eq!(events.len(), 2);

        let f = filters([filter(Some("contract_event"), [(
            &["type"],
            ParsedCheckValue::Equal(json!("order_matched")),
        )])]);
        let events = EventSubscription::get_events(db.db_connection(), 1..=1, f).await;
        assert_eq!(events.len(), 0);

        let f = filters([filter(Some("contract_event"), [(
            &["type"],
            ParsedCheckValue::Contains(vec![json!("order_created"), json!("order_filled")]),
        )])]);
        let events = EventSubscription::get_events(db.db_connection(), 1..=1, f).await;
        assert_eq!(events.len(), 4);

        let f = filters([filter(Some("contract_event"), [
            (&["type"], ParsedCheckValue::Equal(json!("order_filled"))),
            (&["data", "user"], ParsedCheckValue::Equal(json!("rhaki"))),
        ])]);
        let events = EventSubscription::get_events(db.db_connection(), 1..=1, f).await;
        assert_eq!(events.len(), 1);

        let f = filters([filter(Some("contract_event"), [
            (&["type"], ParsedCheckValue::Equal(json!("order_filled"))),
            (&["data", "user"], ParsedCheckValue::Equal(json!("foo"))),
        ])]);
        let events = EventSubscription::get_events(db.db_connection(), 1..=1, f).await;
        assert_eq!(events.len(), 0);

        let f = filters([filter(Some("contract_event"), [
            (&["type"], ParsedCheckValue::Equal(json!("order_created"))),
            (&["data", "user"], ParsedCheckValue::Equal(json!("foo"))),
        ])]);
        let events = EventSubscription::get_events(db.db_connection(), 1..=1, f).await;
        assert_eq!(events.len(), 1);

        let f = filters([
            filter(Some("contract_event"), [
                (&["type"], ParsedCheckValue::Equal(json!("order_created"))),
                (&["data", "user"], ParsedCheckValue::Equal(json!("foo"))),
            ]),
            filter(Some("contract_event"), [
                (&["type"], ParsedCheckValue::Equal(json!("order_filled"))),
                (&["data", "user"], ParsedCheckValue::Equal(json!("rhaki"))),
            ]),
        ]);
        let events = EventSubscription::get_events(db.db_connection(), 1..=1, f).await;
        assert_eq!(events.len(), 2);

        let f = filters([
            filter(Some("contract_event"), [
                (&["type"], ParsedCheckValue::Equal(json!("order_created"))),
                (&["data", "user"], ParsedCheckValue::Equal(json!("foo"))),
            ]),
            filter(Some("transfer"), [(
                &["coins", "usdc"],
                ParsedCheckValue::Equal(json!("1000")),
            )]),
        ]);
        let events = EventSubscription::get_events(db.db_connection(), 1..=1, f).await;
        assert_eq!(events.len(), 2);

        let f = filters([filter(None, [(
            &["data", "user"],
            ParsedCheckValue::Equal(json!("rhaki")),
        )])]);
        let events = EventSubscription::get_events(db.db_connection(), 1..=1, f).await;
        assert_eq!(events.len(), 2);
    }
}
