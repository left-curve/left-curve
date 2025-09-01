use crate::{
    entities::{candle_query::MAX_ITEMS, trade::Trade, trade_query::TradeQueryBuilder},
    error::Result,
};

#[derive(Debug, Default, Eq, PartialEq)]
pub struct TradeCache {
    pub trades: Vec<Trade>,
}

impl TradeCache {
    pub fn compact_keep_n(&mut self, n: usize) {
        if self.trades.len() <= n {
            return;
        }

        self.trades.drain(0..self.trades.len().saturating_sub(n));
    }

    pub async fn preload(&mut self, clickhouse_client: &clickhouse::Client) -> Result<()> {
        self.trades = TradeQueryBuilder::default()
            .with_limit(MAX_ITEMS)
            .fetch_all(clickhouse_client)
            .await?
            .trades;

        Ok(())
    }
}
