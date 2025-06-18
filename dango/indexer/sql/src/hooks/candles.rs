use {
    crate::{error::Error, hooks::Hooks},
    dango_types::{DangoQuerier, dex::OrderFilled},
    grug::{CommitmentStatus, Denom, EventName, EventStatus, EvtCron, JsonDeExt, Udec128},
    grug_app::QuerierProvider,
    indexer_sql::block_to_index::BlockToIndex,
    std::collections::HashMap,
};

impl Hooks {
    pub(crate) async fn save_candles(
        &self,
        block: &BlockToIndex,
        querier: &dyn QuerierProvider,
    ) -> Result<(), Error> {
        let dex = querier.query_dex()?;

        // (base_denom, quote_denom) -> clearing_price
        // Clearing price is denominated as the units of quote asset per 1 unit
        // of the base asset.
        let mut clearing_prices = HashMap::<(Denom, Denom), Udec128>::new();

        // DEX order execution happens exclusively in the end-block cronjob, so
        // we loop through the block's cron outcomes.
        for outcome in &block.block_outcome.cron_outcomes {
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

                    // If this trading pair doesn't have a clearing price recorded
                    // yet, insert it into the map.
                    if !clearing_prices.contains_key(&(base_denom.clone(), quote_denom.clone())) {
                        clearing_prices.insert((base_denom, quote_denom), clearing_price);
                    }
                }
            }
        }

        // TODO: save clearing prices to the database.
        // If for a (base_denom, quote_denom) pair there is no clearing price,
        // meaning no trade occurred for this tracing pair in this block, then
        // the price is the same as the last block's.

        Ok(())
    }
}
