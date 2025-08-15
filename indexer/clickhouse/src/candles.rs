use {
    crate::{
        context::Context,
        entities::{candle_query::MAX_ITEMS, pair_price::PairPrice},
        error::{IndexerError, Result},
        indexer::Indexer,
    },
    chrono::{DateTime, Utc},
    clickhouse::Client,
    dango_types::{
        DangoQuerier,
        dex::{OrderFilled, PairId},
    },
    grug::{
        CommitmentStatus, EventName, EventStatus, EvtCron, FlatCommitmentStatus, FlatEvent,
        FlatEventInfo, FlatEventStatus, JsonDeExt, NaiveFlatten, Number, NumberConst, Udec128_6,
    },
    std::{collections::HashMap, str::FromStr},
};

impl Indexer {
    pub(crate) async fn store_candles(
        clickhouse_client: &Client,
        querier: std::sync::Arc<dyn grug_app::QuerierProvider>,
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

        let dex = querier.as_ref().query_dex()?;

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
            if event.contract != dex {
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

                // We look for the "order filled" event, regardless whether it's
                // a limit order or a market order.
                if event.ty == OrderFilled::EVENT_NAME {
                    #[cfg(feature = "metrics")]
                    metrics::counter!("indexer.clickhouse.order_filled_events.total").increment(1);

                    // Deserialize the event.
                    let order_filled = event.data.clone().deserialize_json::<OrderFilled>()?;

                    // TODO: look at `cleared` field to determine if the order was
                    // fully filled and cleared from the book. If so, we can add
                    // the volume to the map?

                    let pair_id: PairId = (&order_filled).into();

                    let pair_price = pair_prices.entry(pair_id).or_insert(PairPrice {
                        quote_denom: order_filled.quote_denom.to_string(),
                        base_denom: order_filled.base_denom.to_string(),
                        open_price: order_filled.clearing_price,
                        highest_price: order_filled.clearing_price,
                        lowest_price: order_filled.clearing_price,
                        close_price: order_filled.clearing_price,
                        volume_base: Udec128_6::ZERO,
                        volume_quote: Udec128_6::ZERO,
                        created_at: DateTime::<Utc>::from_naive_utc_and_offset(
                            block.info.timestamp.to_naive_date_time(),
                            Utc,
                        ),
                        block_height: block.info.height,
                    });

                    // divide by 2 (because for each buy there's a sell, so it's double counted)
                    let volume_base = order_filled.filled_base.checked_div(two)?;

                    // If the volume overflows, set it to the maximum value.
                    match pair_price.volume_base.checked_add(volume_base) {
                        Ok(volume) => pair_price.volume_base = volume,
                        Err(_) => {
                            // TODO: add sentry error reporting
                            #[cfg(feature = "tracing")]
                            tracing::error!("Overflow in volume_base: {pair_price:#?}");
                            pair_price.volume_base = Udec128_6::MAX;
                        },
                    }

                    // divide by 2 (because for each buy there's a sell, so it's double counted)
                    let volume_quote = order_filled.filled_quote.checked_div(two)?;

                    // If the volume overflows, set it to the maximum value.
                    match pair_price.volume_quote.checked_add(volume_quote) {
                        Ok(volume) => pair_price.volume_quote = volume,
                        Err(_) => {
                            // TODO: add sentry error reporting
                            #[cfg(feature = "tracing")]
                            tracing::error!("Overflow in volume_quote: {pair_price:#?}");
                            pair_price.volume_quote = Udec128_6::MAX;

                            #[cfg(feature = "metrics")]
                            metrics::counter!("indexer.clickhouse.volume_overflow.total")
                                .increment(1);
                        },
                    }

                    if order_filled.clearing_price > pair_price.highest_price {
                        pair_price.highest_price = order_filled.clearing_price;
                    }

                    if order_filled.clearing_price < pair_price.lowest_price {
                        pair_price.lowest_price = order_filled.clearing_price;
                    }

                    // The open price will be overwritten later with the last pair price closing price.
                    // But if we don't have any (first pair price ever), we set it to the clearing
                    // price of the first order filled.
                    // And since we go through the events in reverse order,
                    // the first event is actually the last event in the block.
                    pair_price.open_price = order_filled.clearing_price;

                    // We set the close price to the clearing price of the last but since
                    // we loop through the events in reverse order, the last event
                    // is actually the first event in the block.
                    // pair_price.close_price = order_filled.clearing_price;
                }
            }
        }

        #[cfg(feature = "metrics")]
        metrics::counter!("indexer.clickhouse.pair_prices.processed.total")
            .increment(pair_prices.len() as u64);

        #[cfg(feature = "tracing")]
        tracing::debug!("Saving {} pair prices", pair_prices.len());

        // To reinject previous prices and trigger the clickhouse materialized views
        let mut last_prices = {
            let candle_cache = context.candle_cache.read().await;

            candle_cache
                .pair_price_for_block(block.info.height - 1)
                .cloned()
                .unwrap_or_default()
        };

        // Use Row binary inserter with the official clickhouse serde helpers
        let mut inserter = clickhouse_client
            .inserter::<PairPrice>("pair_prices")?
            .with_max_rows(pair_prices.len() as u64);

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
            }

            inserter.write(pair_price).inspect_err(|_err| {
                #[cfg(feature = "tracing")]
                tracing::error!("Failed to write pair price: {pair_price:#?}: {_err}");
            })?;
        }

        // Manually injecting synthetic pair prices for pairs that
        // didn't have any trades in this block.
        // This is needed to ensure that the materialized views are up to date.
        // We set the open price, lowest price, and highest price to previous
        // closing price, and the volume to zero.
        // The created_at and block_height are set to the current block.
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

            inserter.write(last_price).inspect_err(|_err| {
                #[cfg(feature = "tracing")]
                tracing::error!("Failed to write pair price: {last_price:#?}: {_err}");
            })?;

            #[cfg(feature = "metrics")]
            metrics::counter!("indexer.clickhouse.synthetic_prices.total").increment(1);
        }

        // Do this after looping in the `pair_prices` for `inserter`
        // so `open_price` is set correctly
        {
            last_prices.extend(pair_prices);
            let mut candle_cache = context.candle_cache.write().await;
            candle_cache.add_pair_prices(block.info.height, last_prices);
            candle_cache.compact_keep_n(MAX_ITEMS);
            drop(candle_cache);
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
