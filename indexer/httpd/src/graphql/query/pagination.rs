use {
    crate::context::Context,
    async_graphql::{types::connection::*, *},
    indexer_sql::entity::OrderByBlocks,
    sea_orm::{DatabaseTransaction, EntityTrait, Order, QuerySelect, Select, TransactionTrait},
    serde::{Serialize, de::DeserializeOwned},
    std::{cmp, future::Future, pin::Pin},
};

pub trait CursorFilter<C> {
    fn cursor_filter(self, order: Order, cursor: &C) -> Self;
}

pub async fn paginate_models<C, E, S>(
    app_ctx: &Context,
    after: Option<String>,
    before: Option<String>,
    first: Option<i32>,
    last: Option<i32>,
    sort_by: Option<S>,
    max_items: u64,
    update_query: impl for<'txn> FnOnce(
        Select<E>,
        &'txn DatabaseTransaction,
    ) -> Pin<
        Box<dyn Future<Output = Result<Select<E>, async_graphql::Error>> + Send + 'txn>,
    >,
) -> Result<Connection<OpaqueCursor<C>, E::Model, EmptyFields, EmptyFields>>
where
    C: Send + Sync + Serialize + DeserializeOwned,
    E: EntityTrait,
    <E as EntityTrait>::Model: async_graphql::OutputType,
    Select<E>: OrderByBlocks<E> + CursorFilter<C>,
    C: std::convert::From<<E as EntityTrait>::Model>,
    S: Default,
    sea_orm::Order: std::convert::From<S>,
{
    query_with::<OpaqueCursor<C>, _, _, _, _>(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let mut query = E::find();
                let sort_by = sort_by.unwrap_or_default();
                let order: Order = sort_by.into();

                let limit;

                let mut has_next_page = false;
                let mut has_previous_page = false;

                match (last, &order) {
                    (None, Order::Asc) | (Some(_), Order::Desc) => {
                        query = query.order_by_blocks(Order::Asc)
                    },
                    (None, Order::Desc) | (Some(_), Order::Asc) => {
                        query = query.order_by_blocks(Order::Desc)
                    },
                    _ => {}
                }

                match (first, after, last, before) {
                    (Some(first), None, None, None) => {
                        limit = cmp::min(first as u64, max_items);
                        query = query.limit(limit + 1);
                    },
                    (first, Some(after), None, None) => {
                        query = query.cursor_filter(order.clone(), &after);

                        limit = cmp::min(first.unwrap_or(0) as u64, max_items);
                        query = query.limit(limit + 1);

                        has_previous_page = true;
                    },

                    (None, None, Some(last), None) => {
                        limit = cmp::min(last as u64, max_items);
                        query = query.limit(limit + 1);
                    },

                    (None, None, last, Some(before)) => {
                        match order {
                            Order::Asc => {
                                query = query.cursor_filter(Order::Desc, &before);
                            },
                            Order::Desc => {
                                query = query.cursor_filter(Order::Asc, &before);
                            },
                            Order::Field(_) => {}
                        }

                        limit = cmp::min(last.unwrap_or(0) as u64, max_items);
                        query = query.limit(limit + 1);

                        has_next_page = true;
                    },

                    (None, None, None, None) => {
                        limit = max_items;
                        query = query.limit(max_items + 1);
                    }

                    _ => {
                        return Err(async_graphql::Error::new(
                            "unexpected combination of pagination parameters, should use `first` with `after` or `last` with `before`",
                        ));
                    },
                }

                let txn = app_ctx.db.begin().await?;

                let query = update_query(query, &txn).await?;

                let mut models = query.all(&txn).await?;

                if models.len() > limit as usize {
                    models.pop();
                    if last.is_some() {
                        has_previous_page = true;
                    } else {
                        has_next_page = true;
                    }
                }

                if last.is_some() {
                    models.reverse();
                }

                let mut connection = Connection::new(has_previous_page, has_next_page);
                connection.edges.extend(models.into_iter().map(|model| {
                    Edge::with_additional_fields(
                        OpaqueCursor(model.clone().into()),
                        model,
                        EmptyFields,
                    )
                }));

                Ok::<_, async_graphql::Error>(connection)
            },
        )
        .await
}
