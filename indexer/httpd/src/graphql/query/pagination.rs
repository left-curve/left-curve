use {
    crate::context::Context,
    async_graphql::{types::connection::*, *},
    indexer_sql::entity::OrderByBlocks,
    sea_orm::{EntityTrait, QuerySelect, Select},
    serde::Serialize,
    std::{cmp, future::Future, pin::Pin},
};

pub trait CursorFilter<C> {
    fn apply_cursor_filter(self, sort_by: SortByEnum, cursor: &C) -> Self;
}

#[derive(Debug, Clone, Copy)]
pub enum SortByEnum {
    BlockHeightAsc,
    BlockHeightDesc,
}

pub async fn paginate_models<C, E, S>(
    app_ctx: &Context,
    after: Option<String>,
    before: Option<String>,
    first: Option<i32>,
    last: Option<i32>,
    sort_by: Option<S>,
    max_items: u64,
    update_query: impl FnOnce(
        Select<E>,
    ) -> Pin<
        Box<dyn Future<Output = Result<Select<E>, async_graphql::Error>> + Send>,
    >,
) -> Result<Connection<OpaqueCursor<C>, E::Model, EmptyFields, EmptyFields>>
where
    C: Send + Sync + Serialize + serde::de::DeserializeOwned,
    E: EntityTrait + Send + Sync,
    <E as EntityTrait>::Model: async_graphql::OutputType,
    Select<E>: OrderByBlocks<E> + CursorFilter<C>,
    C: std::convert::From<<E as EntityTrait>::Model>,
    S: Default + Send + Sync + Copy,
    SortByEnum: std::convert::From<S>,
{
    query_with::<OpaqueCursor<C>, _, _, _, _>(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let mut query = E::find();
                let sort_by: SortByEnum = sort_by.unwrap_or_default().into();
                let limit;

                let mut has_next_page = false;
                let mut has_previous_page = false;

                match (last, sort_by) {
                    (None, SortByEnum::BlockHeightAsc) | (Some(_), SortByEnum::BlockHeightDesc) => {
                        query = query.order_by_blocks_asc()
                    },
                    (None, SortByEnum::BlockHeightDesc) | (Some(_), SortByEnum::BlockHeightAsc) => {
                        query = query.order_by_blocks_desc()
                    },
                }

                match (first, after, last, before) {
                    (Some(first), None, None, None) => {
                        limit = cmp::min(first as u64, max_items);
                        query = query.limit(limit + 1);
                    },
                    (first, Some(after), None, None) => {
                        query = query.apply_cursor_filter(sort_by, &after);

                        limit = cmp::min(first.unwrap_or(0) as u64, max_items   );
                        query = query.limit(limit + 1);

                        has_previous_page = true;
                    },

                    (None, None, Some(last), None) => {
                        limit = cmp::min(last as u64, max_items);
                        query = query.limit(limit + 1);
                    },

                    (None, None, last, Some(before)) => {
                        query = match &sort_by {
                            SortByEnum::BlockHeightAsc =>  query.apply_cursor_filter(SortByEnum::BlockHeightDesc, &before),
                            SortByEnum::BlockHeightDesc => query.apply_cursor_filter(SortByEnum::BlockHeightAsc, &before),
                        };

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
                            "Unexpected combination of pagination parameters, should use first with after or last with before",
                        ));
                    },
                }

                let query = update_query(query).await?;

                let mut models = query.all(&app_ctx.db).await?;

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
