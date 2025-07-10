use {
    crate::{
        entities::pair_price::PairPrice,
        error::{IndexerError, Result},
        indexer::Indexer,
    },
    chrono::{DateTime, Utc},
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
                    } = event.data.clone().deserialize_json()?;

                    let pair_id = (base_denom, quote_denom);

                    // If this trading pair doesn't have a clearing price recorded
                    // yet, insert it into the map.
                    clearing_prices.entry(pair_id).or_insert(clearing_price);
                }
            }
        }

        let pairs = clearing_prices
            .into_iter()
            .map(|(pair_id, clearing_price)| PairPrice {
                quote_denom: pair_id.1.to_string(),
                base_denom: pair_id.0.to_string(),
                clearing_price: clearing_price.to_string(),
                created_at: DateTime::<Utc>::from_naive_utc_and_offset(
                    block.info.timestamp.to_naive_date_time(),
                    Utc,
                ),
                block_height: block.info.height,
            })
            .collect::<Vec<_>>();

        #[cfg(feature = "tracing")]
        tracing::info!("Saving {} pair prices", pairs.len());

        // Early return if no pairs to insert
        if pairs.is_empty() {
            return Ok(());
        }

        // Use Row binary inserter with the official clickhouse serde helpers
        let mut inserter = clickhouse_client
            .inserter::<PairPrice>("pair_prices")?
            .with_max_rows(pairs.len() as u64);

        for pair_price in pairs {
            inserter.write(&pair_price)?;
        }

        inserter.commit().await?;
        inserter.end().await?;

        Ok(())
    }
}
