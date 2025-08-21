use {
    crate::{
        context::Context,
        entities::{trade::Trade, trade_query::TradeQueryBuilder},
    },
    async_graphql::{types::connection::*, *},
    grug::Addr,
    serde::{Deserialize, Serialize},
    std::str::FromStr,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TradeCursor {
    block_height: u64,
    trade_idx: u32,
}

impl From<Trade> for TradeCursor {
    fn from(trade: Trade) -> Self {
        Self {
            block_height: trade.block_height,
            trade_idx: trade.trade_idx,
        }
    }
}

#[derive(Default, Debug)]
pub struct TradeQuery;

#[Object]
impl TradeQuery {
    /// Get paginated trades
    async fn trades(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Cursor based pagination")] after: Option<String>,
        #[graphql(desc = "Cursor based pagination")] first: Option<i32>,
        #[graphql(desc = "Account Address")] addr: Option<String>,
    ) -> Result<Connection<OpaqueCursor<TradeCursor>, Trade, EmptyFields, EmptyFields>> {
        let app_ctx = ctx.data::<Context>()?;
        let clickhouse_client = app_ctx.clickhouse_client();

        query_with::<OpaqueCursor<TradeCursor>, _, _, _, _>(
            after,
            None,
            first,
            None,
            |after, _, first, _| async move {
                let mut query_builder = TradeQueryBuilder::default();

                if let Some(addr) = addr {
                    query_builder = query_builder.with_addr(Addr::from_str(&addr)?);
                }

                if let Some(first) = first {
                    query_builder = query_builder.with_limit(first);
                }

                if let Some(after) = after {
                    query_builder =
                        query_builder.with_later_than(after.block_height, after.trade_idx);
                }

                let result = query_builder.fetch_all(clickhouse_client).await?;

                let mut connection =
                    Connection::new(result.has_previous_page, result.has_next_page);

                connection
                    .edges
                    .extend(result.trades.into_iter().map(|trade| {
                        Edge::with_additional_fields(
                            OpaqueCursor(trade.clone().into()),
                            trade,
                            EmptyFields,
                        )
                    }));

                Ok::<_, async_graphql::Error>(connection)
            },
        )
        .await
    }
}
