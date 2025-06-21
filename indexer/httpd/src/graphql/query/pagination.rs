use {
    crate::context::Context,
    async_graphql::{types::connection::*, *},
    sea_orm::{DatabaseTransaction, EntityTrait, QuerySelect, Select, TransactionTrait},
    serde::{Serialize, de::DeserializeOwned},
    std::{cmp, future::Future, pin::Pin},
};

pub trait Reversible {
    fn rev(&self) -> Self;
}

/// A trait to allow filtering a query based on a cursor and an sort
pub trait CursorFilter<S, C> {
    /// Filter the query based on the cursor and sort order.
    fn cursor_filter(self, sort: &S, cursor: &C) -> Self;

    /// Filter the query based on the cursor and sort order in reverse, used for
    /// pagination when going backwards.
    fn cursor_filter_rev(self, sort: &S, cursor: &C) -> Self
    where
        S: Reversible,
        Self: Sized,
    {
        let reversed_sort = sort.rev();
        self.cursor_filter(&reversed_sort, cursor)
    }
}

/// A trait to allow filtering a query based on a cursor and an sort
pub trait CursorOrder<S> {
    /// Order the query based on sort order.
    fn cursor_order(self, sort: S) -> Self;
}

/// Implements cursor-based pagination with bidirectional navigation support.
///
/// # Performance Optimizations
///
/// - **Reverse ordering for `last`**: When paginating backwards, we reverse the sort order
///   and apply the cursor filter in reverse. This allows fetching the last N items efficiently
///   without counting all records or using OFFSET, which would be O(n).
///
/// - **Limit + 1 pattern**: We fetch one extra record beyond the requested limit to determine
///   if there's a next/previous page without an additional COUNT query. The extra record is
///   discarded after setting the appropriate `hasNextPage`/`hasPreviousPage` flags.
///
/// - **Result reversal**: When using `last`, results are fetched in reverse order for efficiency,
///   then reversed again to maintain the expected ordering for the client.
///
/// # Parameters
///
/// Standard Relay cursor pagination parameters:
/// - `first` + `after`: Forward pagination
/// - `last` + `before`: Backward pagination
/// - `sort_by`: Optional sorting configuration with reversible trait for bidirectional queries
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
    Select<E>: CursorFilter<S, C> + CursorOrder<S>,
    C: std::convert::From<<E as EntityTrait>::Model>,
    S: Default + Copy + Reversible,
{
    query_with::<OpaqueCursor<C>, _, _, _, _>(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let mut query = E::find();
                let sort_by = sort_by.unwrap_or_default();

                let limit;

                let mut has_next_page = false;
                let mut has_previous_page = false;

                match last {
                    Some(_) => {
                        query = query.cursor_order(sort_by.rev());
                    }
                    None => {
                        query = query.cursor_order(sort_by);
                    }
                }

                match (first, after, last, before) {
                    (Some(first), None, None, None) => {
                        limit = cmp::min(first as u64, max_items);
                        query = query.limit(limit + 1);
                    },

                    (first, Some(after), None, None) => {
                        query = query.cursor_filter(&sort_by, &after);

                        limit = cmp::min(first.unwrap_or(0) as u64, max_items);
                        query = query.limit(limit + 1);

                        has_previous_page = true;
                    },

                    (None, None, Some(last), None) => {
                        limit = cmp::min(last as u64, max_items);
                        query = query.limit(limit + 1);
                    },

                    (None, None, last, Some(before)) => {
                        query = query.cursor_filter_rev(&sort_by, &before);

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
