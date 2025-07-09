use {
    crate::{entities::pair_price::PairPrice, indexer::Indexer},
    clickhouse::Client,
    dango_types::{DangoQuerier, dex::OrderFilled},
    grug::{CommitmentStatus, EventName, EventStatus, EvtCron, JsonDeExt, Udec128},
    grug_types::Denom,
    std::collections::HashMap,
};

impl Indexer {
    pub(crate) async fn store_candles(
        clickhouse_client: &Client,
        querier: std::sync::Arc<dyn grug_app::QuerierProvider>,
        ctx: &grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        let block = ctx
            .get::<grug_types::Block>()
            .ok_or(grug_app::IndexerError::Database(
                "Block not found".to_string(),
            ))?;

        let block_outcome =
            ctx.get::<grug_types::BlockOutcome>()
                .ok_or(grug_app::IndexerError::Database(
                    "BlockOutcome not found".to_string(),
                ))?;

        let dex = querier.as_ref().query_dex().map_err(|err| {
            grug_app::IndexerError::Generic(format!("Failed to query DEX address: {err}"))
        })?;

        // (base_denom, quote_denom) -> clearing_price
        // Clearing price is denominated as the units of quote asset per 1 unit
        // of the base asset.
        let mut clearing_prices = HashMap::<(Denom, Denom), Udec128>::new();

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
                        ..
                    } = event.data.clone().deserialize_json().map_err(|err| {
                        grug_app::IndexerError::Deserialization(format!(
                            "Failed to deserialize OrderFilled event: {err}",
                        ))
                    })?;

                    let pair_id = (base_denom, quote_denom);

                    // If this trading pair doesn't have a clearing price recorded
                    // yet, insert it into the map.
                    clearing_prices.entry(pair_id).or_insert(clearing_price);
                }
            }
        }

        // TODO: save clearing prices to the database.
        // If for a (base_denom, quote_denom) pair there is no clearing price,
        // meaning no trade occurred for this tracing pair in this block, then
        // the price is the same as the last block's.

        let pairs = clearing_prices
            .into_iter()
            .map(|(pair_id, clearing_price)| PairPrice {
                base_denom: pair_id.0.to_string(),
                quote_denom: pair_id.1.to_string(),
                clearing_price: clearing_price.to_string(),
                created_at: block.info.timestamp.to_naive_date_time(),
                block_height: block.info.height,
            })
            .collect::<Vec<_>>();

        #[cfg(feature = "tracing")]
        tracing::info!("Saving {} pair prices", pairs.len());

        // Code taken from https://github.com/ClickHouse/clickhouse-rs/blob/c48caa3f05de5b2b7a0e33da5a57d621bd13eac8/examples/inserter.rs
        let mut inserter = clickhouse_client
            .inserter::<PairPrice>("pair_prices")
            .map_err(|err| {
                grug_app::IndexerError::Database(format!("Failed to create inserter: {err}"))
            })?
            .with_max_rows(pairs.len() as u64);

        for pair_price in pairs {
            inserter.write(&pair_price).map_err(|err| {
                grug_app::IndexerError::Database(format!("Failed to write pair price: {err}"))
            })?;
            inserter.commit().await.map_err(|err| {
                grug_app::IndexerError::Database(format!("Failed to commit pair price: {err}"))
            })?;
        }

        inserter.end().await.map_err(|err| {
            grug_app::IndexerError::Database(format!("Failed to end inserter: {err}"))
        })?;

        Ok(())
    }
}
