use {
    crate::{entity, entity::perps_trade::PerpsTrade, error::Error},
    dango_types::{
        config::AppConfig,
        perps::{Deleveraged, Liquidated, OrderFilled},
    },
    grug::{
        BlockAndBlockOutcomeWithHttpDetails, CommitmentStatus, EventName, EventStatus, EvtCron,
        FlatCommitmentStatus, FlatEvent, FlatEventInfo, FlatEventStatus, Inner, Json, JsonDeExt,
        NaiveFlatten, SearchEvent, Timestamp,
    },
    indexer_sql::indexer::MAX_ROWS_INSERT,
    itertools::Itertools,
    sea_orm::{EntityTrait, Set, TransactionTrait},
    uuid::Uuid,
};
#[cfg(feature = "metrics")]
use {
    metrics::{counter, describe_counter, describe_histogram, histogram},
    std::time::Instant,
};

const PERPS_EVENT_NAMES: &[&str] = &[
    OrderFilled::EVENT_NAME,
    Liquidated::EVENT_NAME,
    Deleveraged::EVENT_NAME,
];

pub(crate) async fn save_perps_events(
    context: &crate::context::Context,
    block: &BlockAndBlockOutcomeWithHttpDetails,
    app_cfg: Json,
) -> Result<(), Error> {
    #[cfg(feature = "metrics")]
    let start = Instant::now();

    let app_cfg: AppConfig = app_cfg.deserialize_json()?;
    let perps_addr = &app_cfg.addresses.perps;
    let created_at = block.block.info.timestamp.to_naive_date_time();
    let created_at_rfc3339 = Timestamp::from(created_at).to_rfc3339_string();
    let block_height = block.block.info.height;

    let mut events = Vec::new();
    let mut perps_trades = Vec::new();
    let mut idx = 0i32;
    let mut trade_idx = 0u32;

    // Process tx_outcomes (user-submitted transactions).
    for ((_tx, tx_hash), tx_outcome) in block
        .block
        .txs
        .iter()
        .zip(block.block_outcome.tx_outcomes.iter())
    {
        if tx_outcome.result.is_err() {
            continue;
        }

        let flat = tx_outcome.events.clone().flat();

        for event in flat {
            if event.commitment_status != FlatCommitmentStatus::Committed {
                continue;
            }

            let FlatEvent::ContractEvent(ref contract_event) = event.event else {
                continue;
            };

            if contract_event.contract != *perps_addr {
                continue;
            }

            if !PERPS_EVENT_NAMES.contains(&contract_event.ty.as_str()) {
                continue;
            }

            let (user_addr, pair_id) = extract_user_and_pair(contract_event)?;

            // If this is an OrderFilled event, also build a PerpsTrade for the pubsub.
            if contract_event.ty == OrderFilled::EVENT_NAME
                && let Some(trade) = try_build_perps_trade(
                    contract_event,
                    block_height,
                    &created_at_rfc3339,
                    trade_idx,
                )
            {
                perps_trades.push(trade);
                trade_idx += 1;
            }

            events.push(entity::perps_events::ActiveModel {
                id: Set(Uuid::new_v4()),
                idx: Set(idx),
                block_height: Set(block_height as i64),
                tx_hash: Set(tx_hash.to_string()),
                created_at: Set(created_at),
                event_type: Set(contract_event.ty.clone()),
                user_addr: Set(user_addr),
                pair_id: Set(pair_id),
                data: Set(contract_event.data.clone().into_inner()),
            });
            idx += 1;
        }
    }

    // Process cron_outcomes (liquidations happen in cron context).
    for outcome in &block.block_outcome.cron_outcomes {
        let CommitmentStatus::Committed(EventStatus::Ok(EvtCron {
            guest_event: EventStatus::Ok(ref event),
            ..
        })) = outcome.cron_event
        else {
            continue;
        };

        if event.contract != perps_addr {
            continue;
        }

        for flat_event in event
            .clone()
            .naive_flatten(FlatCommitmentStatus::Committed, FlatEventStatus::Ok)
        {
            let FlatEventInfo {
                event: FlatEvent::ContractEvent(ref contract_event),
                commitment_status: FlatCommitmentStatus::Committed,
                event_status: FlatEventStatus::Ok,
                ..
            } = flat_event
            else {
                continue;
            };

            if !PERPS_EVENT_NAMES.contains(&contract_event.ty.as_str()) {
                continue;
            }

            let (user_addr, pair_id) = extract_user_and_pair(contract_event)?;

            // If this is an OrderFilled event, also build a PerpsTrade for the pubsub.
            if contract_event.ty == OrderFilled::EVENT_NAME
                && let Some(trade) = try_build_perps_trade(
                    contract_event,
                    block_height,
                    &created_at_rfc3339,
                    trade_idx,
                )
            {
                perps_trades.push(trade);
                trade_idx += 1;
            }

            events.push(entity::perps_events::ActiveModel {
                id: Set(Uuid::new_v4()),
                idx: Set(idx),
                block_height: Set(block_height as i64),
                tx_hash: Set(String::new()),
                created_at: Set(created_at),
                event_type: Set(contract_event.ty.clone()),
                user_addr: Set(user_addr),
                pair_id: Set(pair_id),
                data: Set(contract_event.data.clone().into_inner()),
            });
            idx += 1;
        }
    }

    // Publish perps trades to subscribers.
    for trade in &perps_trades {
        let _ = context.perps_trade_pubsub.publish(trade.clone()).await;
    }

    #[cfg(feature = "metrics")]
    counter!("indexer.dango.hooks.perps_events.total").increment(events.len() as u64);

    if !events.is_empty() {
        #[cfg(feature = "tracing")]
        tracing::info!(count = events.len(), "Saving perps events");

        let txn = context.db.begin().await?;

        for chunk in events
            .into_iter()
            .chunks(MAX_ROWS_INSERT)
            .into_iter()
            .map(|c| c.collect())
            .collect::<Vec<Vec<_>>>()
        {
            entity::perps_events::Entity::insert_many(chunk)
                .exec_without_returning(&txn)
                .await?;
        }

        txn.commit().await?;
    }

    // Update the in-memory cache after the DB transaction succeeds.
    if !perps_trades.is_empty() {
        let mut cache = context.perps_trade_cache.write().await;
        cache.add_trades(perps_trades);
        cache.compact_keep_n(400);
    }

    #[cfg(feature = "metrics")]
    histogram!("indexer.dango.hooks.perps_events.duration").record(start.elapsed().as_secs_f64());

    Ok(())
}

/// Try to deserialize a contract event into a perps `OrderFilled` and build a
/// `PerpsTrade` suitable for the pubsub + cache.
fn try_build_perps_trade(
    event: &grug_types::CheckedContractEvent,
    block_height: u64,
    created_at: &str,
    trade_idx: u32,
) -> Option<PerpsTrade> {
    let order_filled: OrderFilled = event.data.clone().deserialize_json().ok()?;

    Some(PerpsTrade {
        order_id: order_filled.order_id.to_string(),
        pair_id: order_filled.pair_id.to_string(),
        user: order_filled.user.to_string(),
        fill_price: order_filled.fill_price.to_string(),
        fill_size: order_filled.fill_size.to_string(),
        closing_size: order_filled.closing_size.to_string(),
        opening_size: order_filled.opening_size.to_string(),
        realized_pnl: order_filled.realized_pnl.to_string(),
        fee: order_filled.fee.to_string(),
        created_at: created_at.to_string(),
        block_height,
        trade_idx,
        fill_id: order_filled.fill_id.as_ref().map(ToString::to_string),
        is_maker: order_filled.is_maker,
    })
}

/// Extract `user` and `pair_id` from any of the 3 perps event types.
///
/// All perps events contain `user: Addr` and `pair_id: PairId` fields,
/// so we partially deserialize to extract them.
fn extract_user_and_pair(
    event: &grug_types::CheckedContractEvent,
) -> Result<(String, String), Error> {
    #[derive(serde::Deserialize)]
    struct UserAndPair {
        user: grug::Addr,
        pair_id: grug::Denom,
    }

    let parsed: UserAndPair = event.data.clone().deserialize_json()?;
    Ok((parsed.user.to_string(), parsed.pair_id.to_string()))
}

#[cfg(feature = "metrics")]
pub fn init_metrics() {
    describe_histogram!(
        "indexer.dango.hooks.perps_events.duration",
        "Perps events hook duration in seconds"
    );

    describe_counter!(
        "indexer.dango.hooks.perps_events.total",
        "Total perps events processed"
    );

    describe_counter!(
        "indexer.dango.hooks.perps_events.errors.total",
        "Total perps events hook errors"
    );
}
