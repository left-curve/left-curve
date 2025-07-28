use {
    crate::{
        entities::{CandleInterval, candle_query::CandleQueryBuilder, pair_price::PairPrice},
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
        CommitmentStatus, EventName, EventStatus, EvtCron, JsonDeExt, Number, NumberConst,
        Udec128_6,
    },
    std::{collections::HashMap, str::FromStr, time::Duration},
    strum::IntoEnumIterator,
    tokio::time::sleep,
};

impl Indexer {
    pub(crate) async fn store_candles(
        clickhouse_client: &Client,
        querier: std::sync::Arc<dyn grug_app::QuerierProvider>,
        ctx: &grug_app::IndexerContext,
    ) -> Result<()> {
        let block = ctx
            .get::<grug_types::Block>()
            .ok_or(IndexerError::MissingBlockOrBlockOutcome)?;

        let block_outcome = ctx
            .get::<grug_types::BlockOutcome>()
            .ok_or(IndexerError::MissingBlockOrBlockOutcome)?;

        let dex = querier.as_ref().query_dex()?;

        // Clearing price is denominated as the units of quote asset per 1 unit
        // of the base asset.
        let mut pair_prices = HashMap::<PairId, PairPrice>::new();

        // DEX order execution happens exclusively in the end-block cronjob, so
        // we loop through the block's cron outcomes.
        for outcome in &block_outcome.cron_outcomes {
            // If the event wasn't successful, skip it.
            let CommitmentStatus::Committed(EventStatus::Ok(EvtCron {
                guest_event: EventStatus::Ok(event),
                ..
            })) = &outcome.cron_event
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
            for event in event.contract_events.iter().rev() {
                // We look for the "order filled" event, regardless whether it's
                // a limit order or a market order.
                if event.ty == OrderFilled::EVENT_NAME {
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

                    // If the volume overflows, set it to the maximum value.
                    match pair_price.volume_base.checked_add(order_filled.filled_base) {
                        Ok(volume) => pair_price.volume_base = volume,
                        Err(_) => {
                            // TODO: add sentry error reporting
                            #[cfg(feature = "tracing")]
                            tracing::error!("Overflow in volume_base: {pair_price:#?}");
                            pair_price.volume_base = Udec128_6::MAX;
                        },
                    }

                    // If the volume overflows, set it to the maximum value.
                    match pair_price
                        .volume_quote
                        .checked_add(order_filled.filled_quote)
                    {
                        Ok(volume) => pair_price.volume_quote = volume,
                        Err(_) => {
                            // TODO: add sentry error reporting
                            #[cfg(feature = "tracing")]
                            tracing::error!("Overflow in volume_quote: {pair_price:#?}");
                            pair_price.volume_quote = Udec128_6::MAX;
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

        #[cfg(feature = "tracing")]
        tracing::debug!("Saving {} pair prices", pair_prices.len());

        let last_prices = PairPrice::last_prices(clickhouse_client)
            .await?
            .into_iter()
            .map(|price| Ok(((&price).try_into()?, price)))
            .filter_map(Result::ok)
            .collect::<HashMap<PairId, PairPrice>>();

        // Use Row binary inserter with the official clickhouse serde helpers
        let mut inserter = clickhouse_client
            .inserter::<PairPrice>("pair_prices")?
            .with_max_rows(pair_prices.len() as u64);

        for (_, mut pair_price) in pair_prices.clone().into_iter() {
            // divide by 2 (because for each buy there's a sell, so it's double counted)
            pair_price.volume_base = pair_price
                .volume_base
                .checked_div(Udec128_6::from_str("2.0")?)?;
            pair_price.volume_quote = pair_price
                .volume_quote
                .checked_div(Udec128_6::from_str("2.0")?)?;

            // open price is the closing price of the previous pair price
            if let Some(last_price) = last_prices.get(&(&pair_price).try_into()?) {
                pair_price.open_price = last_price.close_price;
            }

            inserter.write(&pair_price).inspect_err(|_err| {
                #[cfg(feature = "tracing")]
                tracing::error!("Failed to write pair price: {pair_price:#?}: {_err}");
            })?;
        }

        // Manually injecting synthetic pair prices for pairs that
        // didn't have any trades in this block.
        // This is needed to ensure that the materialized views are up to date.
        // We set the open price to zero, and the volume to zero.
        // The created_at and block_height are set to the current block.
        for (_, mut pair_price) in last_prices.into_iter() {
            if pair_prices.contains_key(&(&pair_price).try_into()?) {
                // If the pair price already exists, skip it.
                continue;
            }

            pair_price.volume_base = Udec128_6::ZERO;
            pair_price.volume_quote = Udec128_6::ZERO;
            pair_price.created_at = block.info.timestamp.to_utc_date_time();
            pair_price.block_height = block.info.height;

            inserter.write(&pair_price).inspect_err(|_err| {
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

        // NOTE: we need to check if the materialized view is up to date before we keep going
        // since the notifications are sent based on the materialized view
        for (_, pair_price) in pair_prices.into_iter() {
            for interval in CandleInterval::iter() {
                loop {
                    let max_block_height = CandleQueryBuilder::new(
                        interval,
                        pair_price.base_denom.clone(),
                        pair_price.quote_denom.clone(),
                    )
                    .get_max_block_height(clickhouse_client)
                    .await?;

                    if max_block_height >= block.info.height {
                        break;
                    }

                    #[cfg(feature = "tracing")]
                    tracing::debug!(
                        base_denom = pair_price.base_denom,
                        quote_denom = pair_price.quote_denom,
                        mv_block_height = max_block_height,
                        block_height = block.info.height,
                        "Materialized view for {interval} is not up to date, waiting for it to be updated",
                    );

                    sleep(Duration::from_millis(10)).await;
                }
            }
        }

        PairPrice::cleanup_old_synthetic_data(clickhouse_client, block.info.height).await?;

        Ok(())
    }
}
