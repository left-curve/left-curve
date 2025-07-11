use {
    crate::{
        entities::pair_price::PairPrice,
        error::{IndexerError, Result},
        indexer::Indexer,
    },
    chrono::{DateTime, Utc},
    clickhouse::Client,
    dango_types::{DangoQuerier, dex::OrderFilled},
    grug::{
        CommitmentStatus, EventName, EventStatus, EvtCron, JsonDeExt, Number, NumberConst, Uint128,
    },
    grug_types::Denom,
    std::collections::HashMap,
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
                    let OrderFilled {
                        base_denom,
                        quote_denom,
                        clearing_price,
                        filled_base,
                        filled_quote,
                        ..
                    } = event.data.clone().deserialize_json()?;

                    // TODO: look at `cleared` field to determine if the order was
                    // fully filled and cleared from the book. If so, we can add
                    // the volume to the map?

                    let pair_id = (base_denom.clone(), quote_denom.clone());

                    // If this trading pair doesn't have a clearing price recorded
                    // yet, insert it into the map.

                    let pair_price = pair_prices.entry(pair_id).or_insert(PairPrice {
                        quote_denom: quote_denom.to_string(),
                        base_denom: base_denom.to_string(),
                        clearing_price: clearing_price.into(),
                        volume_base: Uint128::ZERO.into(),
                        volume_quote: Uint128::ZERO.into(),
                        created_at: DateTime::<Utc>::from_naive_utc_and_offset(
                            block.info.timestamp.to_naive_date_time(),
                            Utc,
                        ),
                        block_height: block.info.height,
                    });

                    // If the volume overflows, set it to the maximum value.
                    if pair_price.volume_base.checked_add(filled_base).is_err() {
                        // TODO: add sentry error reporting
                        #[cfg(feature = "tracing")]
                        tracing::error!("Overflow in volume_base: {pair_price:#?}",);
                        pair_price.volume_base = Uint128::MAX.into();
                    };
                    if pair_price.volume_quote.checked_add(filled_quote).is_err() {
                        // TODO: add sentry error reporting
                        #[cfg(feature = "tracing")]
                        tracing::error!("Overflow in volume_quote: {pair_price:#?}",);
                        pair_price.volume_quote = Uint128::MAX.into();
                    };
                }
            }
        }

        #[cfg(feature = "tracing")]
        tracing::info!("Saving {} pair prices", pair_prices.len());

        // Early return if no pairs to insert
        if pair_prices.is_empty() {
            return Ok(());
        }

        // Use Row binary inserter with the official clickhouse serde helpers
        let mut inserter = clickhouse_client
            .inserter::<PairPrice>("pair_prices")?
            .with_max_rows(pair_prices.len() as u64);

        for (_, mut pair_price) in pair_prices.into_iter() {
            // divide by 2 (because for each buy there's a sell, so it's double counted)
            pair_price.volume_base /= Uint128::from(2).into();
            pair_price.volume_quote /= Uint128::from(2).into();
            inserter.write(&pair_price).inspect_err(|_err| {
                #[cfg(feature = "tracing")]
                tracing::error!("Failed to write pair price: {pair_price:#?}: {_err}",);
            })?;
        }

        inserter.commit().await.inspect_err(|_err| {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to commit inserted for pair prices: : {_err}",);
        })?;
        inserter.end().await.inspect_err(|_err| {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to end inserter for pair prices: {_err}",);
        })?;

        Ok(())
    }
}
