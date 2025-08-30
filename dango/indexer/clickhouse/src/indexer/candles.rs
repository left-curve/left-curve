use {
    crate::{
        context::Context,
        entities::pair_price::PairPrice,
        error::{IndexerError, Result},
        indexer::Indexer,
    },
    chrono::{DateTime, Utc},
    dango_types::dex::OrdersMatched,
    grug::{
        Addr, CommitmentStatus, EventName, EventStatus, EvtCron, FlatCommitmentStatus, FlatEvent,
        FlatEventInfo, FlatEventStatus, JsonDeExt, NaiveFlatten, Number,
    },
};

pub mod cache;
pub mod generator;

impl Indexer {
    pub(crate) async fn store_candles(
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

        let created_at = DateTime::<Utc>::from_naive_utc_and_offset(
            block.info.timestamp.to_naive_date_time(),
            Utc,
        );

        // Clearing price is denominated as the units of quote asset per 1 unit
        // of the base asset.
        let mut pair_prices = Vec::new();

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

                    let volume_quote = order_matched
                        .volume
                        .checked_mul(order_matched.clearing_price)?;

                    pair_prices.push(PairPrice {
                        quote_denom: order_matched.quote_denom.to_string(),
                        base_denom: order_matched.base_denom.to_string(),
                        clearing_price: order_matched.clearing_price,
                        volume_base: order_matched.volume,
                        volume_quote,
                        created_at,
                        block_height: block.info.height,
                    });
                }
            }
        }

        let candle_generator = generator::CandleGenerator::new(context.clone());

        candle_generator
            .add_pair_prices(block.info.height, created_at, pair_prices)
            .await
    }
}
