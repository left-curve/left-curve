use {
    super::MAX_PAST_BLOCKS,
    async_graphql::{futures_util::stream::Stream, *},
    futures_util::stream::{StreamExt, once},
    indexer_sql::entity,
    itertools::Itertools,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder},
    std::{collections::VecDeque, ops::RangeInclusive},
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

fn verify_json(json: &serde_json::Value, filter: &ParsedFilterData) -> bool {
    fn recursive(
        mut keys: VecDeque<String>,
        value: &ParsedCheckValue,
        json: &serde_json::Value,
    ) -> bool {
        let Some(key) = keys.pop_front() else {
            return match value {
                ParsedCheckValue::Equal(value) => value == json,
                ParsedCheckValue::Contains(values) => values.contains(json),
            };
        };

        match json {
            serde_json::Value::Array(values) => {
                let Ok(key) = key.parse::<usize>() else {
                    return false;
                };

                let Some(json) = values.get(key) else {
                    return false;
                };

                recursive(keys, value, json)
            },
            serde_json::Value::Object(map) => {
                let Some(json) = map.get(&key) else {
                    return false;
                };

                recursive(keys, value, json)
            },
            _ => false,
        }
    }

    recursive(filter.path.clone(), &filter.value, json)
}

#[derive(Default)]
pub struct EventSubscription;

impl EventSubscription {
    async fn get_events(
        app_ctx: &crate::context::Context,
        block_heights: RangeInclusive<i64>,
        filter: Option<Vec<ParsedFilter>>,
    ) -> Vec<entity::events::Model> {
        let mut query = entity::events::Entity::find()
            .order_by_asc(entity::events::Column::BlockHeight)
            .order_by_asc(entity::events::Column::EventIdx)
            .filter(entity::events::Column::BlockHeight.is_in(block_heights));

        if let Some(filters) = &filter {
            for filter in filters {
                if let Some(r#type) = &filter.r#type {
                    query = query.filter(entity::events::Column::Type.eq(r#type));
                }
            }
        }

        let events = query
            .all(&app_ctx.db)
            .await
            .inspect_err(|_e| {
                #[cfg(feature = "tracing")]
                tracing::error!(%_e, "`get_events` error");
            })
            .unwrap_or_default();

        if let Some(filters) = filter {
            events
                .into_iter()
                .filter(|event| {
                    'a: for filter in &filters {
                        if let Some(r#type) = &filter.r#type {
                            if &event.r#type != r#type {
                                continue;
                            }
                        }

                        if let Some(data) = &filter.data {
                            for data in data {
                                if !verify_json(&event.data, data) {
                                    continue 'a;
                                }
                            }
                        }

                        return true;
                    }

                    false
                })
                .collect()
        } else {
            events
        }
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

        let block_range = match since_block_height {
            Some(block_height) => block_height as i64..=latest_block_height,
            None => latest_block_height..=latest_block_height,
        };

        if block_range.try_len().unwrap_or(0) > MAX_PAST_BLOCKS {
            return Err(async_graphql::Error::new("`since_block_height` is too old"));
        }

        let filter = if let Some(filter) = filter {
            Some(
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
                                                                "`filter` is invalid",
                                                            ));
                                                        }
                                                    },
                                                    CheckValue::Contains => {
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
                    .collect::<Result<Vec<_>, async_graphql::Error>>()?,
            )
        } else {
            None
        };

        let f_filter = filter.clone();

        Ok(
            once(async move { Self::get_events(app_ctx, block_range, f_filter).await })
                .chain(
                    app_ctx
                        .pubsub
                        .subscribe_block_minted()
                        .await?
                        .then(move |block_height| {
                            let filter = filter.clone();
                            async move {
                                Self::get_events(
                                    app_ctx,
                                    block_height as i64..=block_height as i64,
                                    filter,
                                )
                                .await
                            }
                        }),
                )
                .filter_map(|events| async move {
                    if events.is_empty() {
                        None
                    } else {
                        Some(events)
                    }
                }),
        )
    }
}

#[cfg(test)]
mod tests {
    use {super::*, serde_json::json};

    fn filterdata(path: &[&str], value: serde_json::Value) -> ParsedFilterData {
        ParsedFilterData {
            path: path.iter().map(|s| s.to_string()).collect(),
            value: ParsedCheckValue::Equal(value),
        }
    }

    fn filterdata_contains(path: &[&str], value: &[serde_json::Value]) -> ParsedFilterData {
        ParsedFilterData {
            path: path.iter().map(|s| s.to_string()).collect(),
            value: ParsedCheckValue::Contains(value.to_vec()),
        }
    }

    #[test]
    fn test_verify() {
        let json = json!({
            "a": {
                "b": {
                    "c": 1
                },
                "d": [1,2,3],
                "e" : "hello",
            }
        });

        // True

        let filter = filterdata(&["a", "b", "c"], json!(1));
        assert!(verify_json(&json, &filter));

        let filter = filterdata(&["a", "e"], json!("hello"));
        assert!(verify_json(&json, &filter));

        let filter = filterdata(&["a", "d"], json!([1, 2, 3]));
        assert!(verify_json(&json, &filter));

        let filter = filterdata(&["a", "d", "0"], json!(1));
        assert!(verify_json(&json, &filter));

        let filter = filterdata(&["a", "d", "2"], json!(3));
        assert!(verify_json(&json, &filter));

        let filter = filterdata(
            &["a", "b"],
            json!({
                "c": 1
            }),
        );
        assert!(verify_json(&json, &filter));

        let filter = filterdata(
            &[],
            json!({
                "a": {
                    "b": {
                        "c": 1
                    },
                    "d": [1,2,3],
                    "e" : "hello",
                }
            }),
        );
        assert!(verify_json(&json, &filter));

        let filter = filterdata_contains(&["a", "b", "c"], &[json!(1), json!(2)]);
        assert!(verify_json(&json, &filter));

        // False

        let filter = filterdata(&["a", "b", "c"], json!("1"));
        assert!(!verify_json(&json, &filter));

        let filter = filterdata(&["a", "b", "c"], json!(2));
        assert!(!verify_json(&json, &filter));

        let filter = filterdata(&["a", "b", "c", "f"], json!(2));
        assert!(!verify_json(&json, &filter));

        let filter = filterdata(&["a", "d", "0"], json!(2));
        assert!(!verify_json(&json, &filter));

        let filter = filterdata_contains(&["a", "b", "c"], &[json!(2), json!(3)]);
        assert!(!verify_json(&json, &filter));
    }
}
