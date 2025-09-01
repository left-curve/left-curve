use {
    crate::{
        context::Context,
        entities::{candle_query::MAX_ITEMS, trade::Trade},
        error::{IndexerError, Result},
        indexer::Indexer,
    },
    chrono::{DateTime, Utc},
    dango_types::dex::OrderFilled,
    grug::{
        Addr, CommitmentStatus, EventName, EventStatus, EvtCron, FlatCommitmentStatus, FlatEvent,
        FlatEventInfo, FlatEventStatus, JsonDeExt, NaiveFlatten,
    },
};

pub mod cache;

impl Indexer {
    pub(crate) async fn store_trades(
        dex_addr: &Addr,
        ctx: &grug_app::IndexerContext,
        context: &Context,
    ) -> Result<()> {
        let clickhouse_client = context.clickhouse_client().clone();

        let block = ctx
            .get::<grug_types::Block>()
            .ok_or(IndexerError::MissingBlockOrBlockOutcome)?;

        let block_outcome = ctx
            .get::<grug_types::BlockOutcome>()
            .ok_or(IndexerError::MissingBlockOrBlockOutcome)?
            .clone();

        // Clearing price is denominated as the units of quote asset per 1 unit
        // of the base asset.
        let mut trades = Vec::new();

        let mut trade_idx = 0;

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

                    let trade = Trade {
                        addr: order_filled.user.to_string(),
                        quote_denom: order_filled.quote_denom.to_string(),
                        base_denom: order_filled.base_denom.to_string(),
                        direction: order_filled.direction,
                        order_type: order_filled.kind,
                        filled_base: order_filled.filled_base,
                        filled_quote: order_filled.filled_quote,
                        refund_base: order_filled.refund_base,
                        refund_quote: order_filled.refund_quote,
                        fee_base: order_filled.fee_base,
                        fee_quote: order_filled.fee_quote,
                        clearing_price: order_filled.clearing_price,
                        created_at: DateTime::<Utc>::from_naive_utc_and_offset(
                            block.info.timestamp.to_naive_date_time(),
                            Utc,
                        ),
                        block_height: block.info.height,
                        trade_idx,
                    };

                    trade_idx += 1;

                    trades.push(trade.clone());
                    context.trade_pubsub.publish(trade).await?;
                }
            }
        }

        #[cfg(feature = "metrics")]
        metrics::counter!("indexer.clickhouse.trades.processed.total")
            .increment(trades.len() as u64);

        #[cfg(feature = "tracing")]
        tracing::debug!("Saving {} trades", trades.len());

        // Use Row binary inserter with the official clickhouse serde helpers
        let mut inserter = clickhouse_client
            .inserter::<Trade>("trades")?
            .with_max_rows(trades.len() as u64);

        for trade in trades.iter() {
            inserter.write(trade).inspect_err(|_err| {
                #[cfg(feature = "tracing")]
                tracing::error!("Failed to write trade: {trade:#?}: {_err}");
            })?;
        }

        inserter.commit().await.inspect_err(|_err| {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to commit inserter for trades: {_err}");
        })?;
        inserter.end().await.inspect_err(|_err| {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to end inserter for trades: {_err}");
        })?;

        let mut trade_cache = context.trade_cache.write().await;
        trade_cache.trades.append(&mut trades);
        trade_cache.compact_keep_n(MAX_ITEMS * 2);

        Ok(())
    }
}
