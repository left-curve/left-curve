use {
    crate::{
        entities::{candle_query::MAX_ITEMS, trade::Trade, trade_query::TradeQueryBuilder},
        error::{IndexerError, Result},
    },
    dango_types::dex::PairId,
    std::collections::HashMap,
};

#[derive(Debug, Default, Eq, PartialEq)]
pub struct TradeCache {
    pub trades: HashMap<PairId, Vec<Trade>>,
}

impl TradeCache {
    pub fn trades_for_pair(&self, pair: &PairId) -> Option<&Vec<Trade>> {
        self.trades.get(pair)
    }

    pub fn add_trades(&mut self, trades: Vec<Trade>) -> Result<()> {
        for trade in trades {
            self.trades
                .entry((&trade).try_into()?)
                .or_default()
                .push(trade);
        }

        Ok(())
    }

    pub fn compact_keep_n(&mut self, n: usize) {
        for trades in self.trades.values_mut() {
            if trades.len() > n {
                trades.drain(0..trades.len() - n);
            }
        }
    }

    pub async fn preload(&mut self, clickhouse_client: &clickhouse::Client) -> Result<()> {
        let trades = TradeQueryBuilder::default()
            .with_limit(MAX_ITEMS)
            .fetch_all(clickhouse_client)
            .await?
            .trades
            .into_iter()
            .try_fold(
                HashMap::new(),
                |mut acc: std::collections::HashMap<_, Vec<Trade>>, trade| {
                    let pair = (&trade).try_into()?;
                    acc.entry(pair).or_default().push(trade);
                    Ok::<_, IndexerError>(acc)
                },
            )?;

        self.trades = trades;

        Ok(())
    }
}
