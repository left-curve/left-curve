use {
    crate::{
        entities::{candle_query::MAX_ITEMS, trade::Trade, trade_query::TradeQueryBuilder},
        error::Result,
    },
    itertools::Itertools,
    std::collections::HashMap,
};

#[derive(Debug, Default, Eq, PartialEq)]
pub struct TradeCache {
    pub trades: HashMap<u64, Vec<Trade>>,
}

impl TradeCache {
    pub fn compact_keep_n(&mut self, n: usize) {
        if self.trades.len() <= n {
            return;
        }

        let mut keys_to_remove: Vec<u64> = self.trades.keys().cloned().collect();
        keys_to_remove.sort();

        let remove_count = self.trades.len() - n;
        for key in keys_to_remove.into_iter().take(remove_count) {
            self.trades.remove(&key);
        }
    }

    pub async fn preload(&mut self, clickhouse_client: &clickhouse::Client) -> Result<()> {
        self.trades = TradeQueryBuilder::default()
            .with_limit(MAX_ITEMS)
            .fetch_all(clickhouse_client)
            .await?
            .trades
            .into_iter()
            .into_group_map_by(|trade| trade.block_height);

        Ok(())
    }
}
