#![allow(unused_variables)]
#![allow(dead_code)]

use {
    crate::{
        entities::{CandleInterval, candle::Candle, pair_price::PairPrice},
        error::Result,
    },
    chrono::{DateTime, Utc},
    clickhouse::Client,
    dango_types::dex::PairId,
    grug::{Denom, NumberConst, Udec128_6},
    std::{
        collections::{HashMap, HashSet},
        str::FromStr,
    },
    strum::IntoEnumIterator,
};

pub struct PriceBackfiller {
    client: Client,
}

impl PriceBackfiller {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn backfill_intervals(
        &self,
        block_height: u64,
        created_at: DateTime<Utc>,
    ) -> Result<()> {
        let last_prices = PairPrice::last_prices(&self.client)
            .await?
            .into_iter()
            .map(|price| {
                Ok((
                    PairId {
                        base_denom: Denom::from_str(&price.base_denom)?,
                        quote_denom: Denom::from_str(&price.quote_denom)?,
                    },
                    price,
                ))
            })
            .filter_map(Result::ok)
            .collect::<HashMap<PairId, PairPrice>>();

        for interval in CandleInterval::iter() {
            let missing_pairs =
                Candle::get_missing_pairs(interval, &self.client, block_height).await?;

            self.insert_synthetic_candles(
                interval,
                &missing_pairs,
                &last_prices,
                block_height,
                created_at,
            )
            .await?;
        }

        Ok(())
    }

    async fn insert_synthetic_candles(
        &self,
        interval: CandleInterval,
        missing_pairs: &[PairId],
        last_prices: &HashMap<PairId, PairPrice>,
        block_height: u64,
        created_at: DateTime<Utc>,
    ) -> Result<()> {
        if missing_pairs.is_empty() {
            return Ok(());
        }

        // for missing_pair in missing_pairs.into_iter() {
        //     let Some(pair_price) = last_prices.get(missing_pair) else {
        //         #[cfg(feature = "tracing")]
        //         tracing::warn!(
        //             "No last price found for missing pair: {missing_pair:?}, skipping candle insertion"
        //         );
        //         continue;
        //     };
        // }

        Ok(())
    }

    /// Call this for each new block to ensure all pairs have price data
    pub async fn backfill_missing_prices(&self, current_block: u64) -> Result<()> {
        // Get all pairs that had activity in previous blocks
        let all_pairs = self.get_all_known_pairs().await?;

        // Get pairs that have activity in current block
        let active_pairs = self.get_active_pairs_for_block(current_block).await?;

        // Find pairs missing in current block
        let missing_pairs: Vec<_> = all_pairs.difference(&active_pairs).collect();

        if missing_pairs.is_empty() {
            return Ok(());
        }

        // Get last known prices for missing pairs
        let last_prices = self.get_last_prices_for_pairs(&missing_pairs).await?;

        // Insert synthetic records
        self.insert_synthetic_prices(last_prices, current_block)
            .await?;

        Ok(())
    }

    async fn get_all_known_pairs(&self) -> Result<HashSet<(String, String)>> {
        let query = "SELECT DISTINCT quote_denom, base_denom FROM pair_prices";
        let rows: Vec<(String, String)> = self.client.query(query).fetch_all().await?;

        Ok(rows.into_iter().collect())
    }

    async fn get_active_pairs_for_block(
        &self,
        block_height: u64,
    ) -> Result<HashSet<(String, String)>> {
        let query =
            "SELECT DISTINCT quote_denom, base_denom FROM pair_prices WHERE block_height = ?";
        let rows: Vec<(String, String)> = self
            .client
            .query(query)
            .bind(block_height)
            .fetch_all()
            .await?;

        Ok(rows.into_iter().collect())
    }

    async fn get_last_prices_for_pairs(
        &self,
        pairs: &[&(String, String)],
    ) -> Result<Vec<PairPrice>> {
        if pairs.is_empty() {
            return Ok(vec![]);
        }

        // Build WHERE clause for multiple pairs efficiently
        let conditions: Vec<String> = pairs
            .iter()
            .map(|(quote, base)| format!("(quote_denom = '{quote}' AND base_denom = '{base}')"))
            .collect();

        let query = format!(
            r#"
            SELECT
                quote_denom,
                base_denom,
                argMax(clearing_price, created_at) as last_price,
                argMax(volume_base, created_at) as last_volume_base,
                argMax(volume_quote, created_at) as last_volume_quote,
                max(created_at) as last_time,
                max(block_height) as block_height
            FROM pair_prices
            WHERE ({})
            GROUP BY quote_denom, base_denom
            "#,
            conditions.join(" OR ")
        );

        Ok(self.client.query(&query).fetch_all().await?)
    }

    async fn insert_synthetic_prices(
        &self,
        prices: Vec<PairPrice>,
        block_height: u64,
    ) -> Result<()> {
        if prices.is_empty() {
            return Ok(());
        }

        // TODO: use the block.created_at timestamp instead of now
        let now = chrono::Utc::now();

        let mut inserter = self
            .client
            .inserter::<PairPrice>("pair_prices")?
            .with_max_rows(prices.len() as u64);

        for mut pair_price in prices.into_iter() {
            // divide by 2 (because for each buy there's a sell, so it's double counted)
            pair_price.volume_base = Udec128_6::ZERO;
            pair_price.volume_quote = Udec128_6::ZERO;
            pair_price.created_at = now;
            pair_price.block_height = block_height;

            inserter.write(&pair_price).inspect_err(|_err| {
                #[cfg(feature = "tracing")]
                tracing::error!("Failed to write pair price: {pair_price:#?}: {_err}",);
            })?;
        }

        inserter.commit().await.inspect_err(|_err| {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to commit inserter for pair prices: {_err}",);
        })?;

        inserter.end().await.inspect_err(|_err| {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to end inserter for pair prices: {_err}",);
        })?;

        Ok(())
    }
}
