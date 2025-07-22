use {
    crate::{
        entities::{CandleInterval, candle_query::CandleQueryBuilder, pair_price::PairPrice},
        error::{IndexerError, Result},
        indexer::Indexer,
    },
    chrono::{DateTime, Utc},
    clickhouse::Client,
    dango_types::{DangoQuerier, dex::OrderFilled},
    grug::{
        CommitmentStatus, EventName, EventStatus, EvtCron, JsonDeExt, Number, NumberConst,
        Udec128_6,
    },
    grug_types::Denom,
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

        // (base_denom, quote_denom) -> clearing_price
        // Clearing price is denominated as the units of quote asset per 1 unit
        // of the base asset.
        let mut pair_prices = HashMap::<(Denom, Denom), PairPrice>::new();

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
            // trading pair, its clearing price is determined by the last executed
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

                    let pair_id = (
                        order_filled.base_denom.clone(),
                        order_filled.quote_denom.clone(),
                    );

                    // If this trading pair doesn't have a clearing price recorded
                    // yet, insert it into the map.

                    let pair_price = pair_prices.entry(pair_id).or_insert(PairPrice {
                        quote_denom: order_filled.quote_denom.to_string(),
                        base_denom: order_filled.base_denom.to_string(),
                        clearing_price: order_filled.clearing_price,
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
                            tracing::error!("Overflow in volume_base: {pair_price:#?}",);
                            pair_price.volume_base = Udec128_6::MAX;
                        },
                    }

                    match pair_price
                        .volume_quote
                        .checked_add(order_filled.filled_quote)
                    {
                        Ok(volume) => pair_price.volume_quote = volume,
                        Err(_) => {
                            // TODO: add sentry error reporting
                            #[cfg(feature = "tracing")]
                            tracing::error!("Overflow in volume_quote: {pair_price:#?}",);
                            pair_price.volume_quote = Udec128_6::MAX;
                        },
                    }
                }
            }
        }

        #[cfg(feature = "tracing")]
        tracing::debug!("Saving {} pair prices", pair_prices.len());

        // Early return if no pairs to insert
        if pair_prices.is_empty() {
            return Ok(());
        }

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

        Ok(())
    }
}
