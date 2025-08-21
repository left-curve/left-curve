use {
    crate::{
        context::Context,
        entities::{candle_query::MAX_ITEMS, pair_price::PairPrice},
        error::{IndexerError, Result},
        indexer::Indexer,
    },
    chrono::{DateTime, Utc},
    clickhouse::Client,
    dango_types::dex::{OrderFilled, OrdersMatched, PairId},
    grug::{
        Addr, CommitmentStatus, EventName, EventStatus, EvtCron, FlatCommitmentStatus, FlatEvent,
        FlatEventInfo, FlatEventStatus, JsonDeExt, NaiveFlatten, Number, NumberConst, Udec128_6,
    },
    std::{collections::HashMap, str::FromStr, time::Duration},
    tokio::time::sleep,
};

impl Indexer {
    pub(crate) async fn store_candles(
        clickhouse_client: &Client,
        dex_addr: &Addr,
        ctx: &grug_app::IndexerContext,
        context: &Context,
    ) -> Result<()> {
        let block = ctx
            .get::<grug_types::Block>()
            .ok_or(IndexerError::MissingBlockOrBlockOutcome)?;

        let block_outcome = ctx
            .get::<grug_types::BlockOutcome>()
            .ok_or(IndexerError::MissingBlockOrBlockOutcome)?
            .clone();

        // Clearing price is denominated as the units of quote asset per 1 unit
        // of the base asset.
        let mut pair_prices = HashMap::<PairId, PairPrice>::new();
        let two = Udec128_6::from_str("2.0")?;

        // DEX order execution happens exclusively in the end-block cronjob, so
        // we loop through the block's cron outcomes.
        for outcome in block_outcome.cron_outcomes {
            // If the event wasn't successful, skip it.
            let CommitmentStatus::Committed(EventStatus::Ok(EvtCron {
                guest_event: EventStatus::Ok(event),
                ..
            })) = outcome.cron_event
            else {
                continue;
            };

            // If the event wasn't emitted by the DEX, skip it.
            if event.contract != dex_addr {
                continue;
            }

            // Loop through the DEX events in the reverse order. Meaning, for each
            // trading pair, its closing price is determined by the last executed
            // order in this block.
            for event in event
                .naive_flatten(FlatCommitmentStatus::Committed, FlatEventStatus::Ok)
                .into_iter()
                .rev()
            {
                let FlatEventInfo {
                    event: FlatEvent::ContractEvent(event),
                    commitment_status: FlatCommitmentStatus::Committed,
                    event_status: FlatEventStatus::Ok,
                    ..
                } = event
                else {
                    continue;
                };

                if event.ty == OrdersMatched::EVENT_NAME {
                    #[cfg(feature = "metrics")]
                    metrics::counter!("indexer.clickhouse.order_matched_events.total").increment(1);

                    // Deserialize the event.
                    let order_matched = event.data.clone().deserialize_json::<OrdersMatched>()?;

                    let pair_id: PairId = (&order_matched).into();

                    let volume_quote = order_matched
                        .volume
                        .checked_mul(order_matched.clearing_price)?;

                    let _pair_price = pair_prices.entry(pair_id).or_insert(PairPrice {
                        quote_denom: order_matched.quote_denom.to_string(),
                        base_denom: order_matched.base_denom.to_string(),
                        open_price: order_matched.clearing_price,
                        highest_price: order_matched.clearing_price,
                        lowest_price: order_matched.clearing_price,
                        close_price: order_matched.clearing_price,
                        volume_base: order_matched.volume,
                        volume_quote,
                        created_at: DateTime::<Utc>::from_naive_utc_and_offset(
                            block.info.timestamp.to_naive_date_time(),
                            Utc,
                        ),
                        block_height: block.info.height,
                    });
                }
            }
        }

        #[cfg(feature = "metrics")]
        metrics::counter!("indexer.clickhouse.pair_prices.processed.total")
            .increment(pair_prices.len() as u64);

        #[cfg(feature = "tracing")]
        tracing::debug!("Saving {} pair prices", pair_prices.len());

        // NOTE: `store_candles` isn't necessarily called in order, during tests with
        // very fast calls, those won't come in order. Could also potentially
        // happen in production (but rare I guess).
        // We must have a previous price, unless it's genesis block. Because the previous
        // block close price defines this block open price.
        // Previous prices are also used to trigger clickhouse materialized views
        // when no new price have arrived in this current block.

        let mut last_prices = {
            let mut previous_prices = None;

            // NOTE: I'm avoiding genesis block since we know no previous prices exist.
            if block.info.height > 1 {
                // Preventing infinite loop
                for _ in 0..=10 {
                    let candle_cache = context.candle_cache.read().await;
                    previous_prices = candle_cache
                        .pair_price_for_block(block.info.height - 1)
                        .cloned();
                    drop(candle_cache);

                    // NOTE: could maybe look at the clickhouse data as a fallback if not in memory,
                    // which could happen when restarting the indexer.
                    if previous_prices.is_some() {
                        break;
                    }

                    sleep(Duration::from_millis(100)).await;
                }
            }

            previous_prices.unwrap_or_default()
        };

        // Changing open price to the closing price of the previous pair price
        for (pair_id, pair_price) in pair_prices.iter_mut() {
            // open price is the closing price of the previous pair price
            if let Some(last_price) = last_prices.get(pair_id) {
                pair_price.open_price = last_price.close_price;

                if last_price.close_price > pair_price.highest_price {
                    pair_price.highest_price = last_price.close_price;
                }

                if last_price.close_price < pair_price.lowest_price {
                    pair_price.lowest_price = last_price.close_price;
                }

                #[cfg(feature = "tracing")]
                tracing::debug!(block.info.height, "found previous pair price");
            } else {
                #[cfg(feature = "tracing")]
                tracing::debug!(block.info.height, "didn't find previous pair price");
            }
        }

        for (pair_id, last_price) in last_prices.iter_mut() {
            if pair_prices.contains_key(pair_id) {
                // If the pair price already exists, skip it.
                continue;
            }

            last_price.volume_base = Udec128_6::ZERO;
            last_price.volume_quote = Udec128_6::ZERO;

            last_price.created_at = block.info.timestamp.to_utc_date_time();
            last_price.block_height = block.info.height;

            last_price.open_price = last_price.close_price;
            last_price.lowest_price = last_price.close_price;
            last_price.highest_price = last_price.close_price;
        }

        let mut all_pair_prices = last_prices;

        all_pair_prices.extend(pair_prices);

        // Writing the cache asap so other threads needing this have it asap.
        {
            let mut candle_cache = context.candle_cache.write().await;
            candle_cache.add_pair_prices(block.info.height, all_pair_prices.clone());
            candle_cache.compact_keep_n(MAX_ITEMS * 2);
            drop(candle_cache);
        }

        // Use Row binary inserter with the official clickhouse serde helpers
        let mut inserter = clickhouse_client
            .inserter::<PairPrice>("pair_prices")?
            .with_max_rows(all_pair_prices.len() as u64);

        for (_, pair_price) in all_pair_prices.iter() {
            #[cfg(feature = "tracing")]
            tracing::debug!(
                "{} Inserting pair price: open {} close {}",
                pair_price.block_height,
                pair_price.open_price,
                pair_price.close_price
            );

            inserter.write(pair_price).inspect_err(|_err| {
                #[cfg(feature = "tracing")]
                tracing::error!("Failed to write pair price: {pair_price:#?}: {_err}");
            })?;
        }

        inserter.commit().await.inspect_err(|_err| {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to commit inserter for pair prices: {_err}");
        })?;
        inserter.end().await.inspect_err(|_err| {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to end inserter for pair prices: {_err}");
        })?;

        Ok(())
    }
}
