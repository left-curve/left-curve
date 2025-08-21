use {
    crate::{
        context::Context,
        entities::{candle_query::MAX_ITEMS, pair_price::PairPrice},
        error::{IndexerError, Result},
        indexer::Indexer,
    },
    chrono::{DateTime, Utc},
    clickhouse::Client,
    dango_types::dex::{OrdersMatched, PairId},
    grug::{
        Addr, CommitmentStatus, EventName, EventStatus, EvtCron, FlatCommitmentStatus, FlatEvent,
        FlatEventInfo, FlatEventStatus, JsonDeExt, NaiveFlatten, Number,
    },
    std::collections::HashMap,
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

                    pair_prices.insert(pair_id, PairPrice {
                        quote_denom: order_matched.quote_denom.to_string(),
                        base_denom: order_matched.base_denom.to_string(),
                        clearing_price: order_matched.clearing_price,
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

        // Writing the cache asap so other threads needing this have it asap.
        {
            let mut candle_cache = context.candle_cache.write().await;
            candle_cache.add_pair_prices(block.info.height, pair_prices.clone());
            candle_cache.compact_keep_n(MAX_ITEMS * 2);
            drop(candle_cache);
        }

        // Use Row binary inserter with the official clickhouse serde helpers
        let mut inserter = clickhouse_client
            .inserter::<PairPrice>("pair_prices")?
            .with_max_rows(pair_prices.len() as u64);

        for (_, pair_price) in pair_prices.iter() {
            #[cfg(feature = "tracing")]
            tracing::debug!(
                "{} Inserting pair price: {}",
                pair_price.block_height,
                pair_price.clearing_price,
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
