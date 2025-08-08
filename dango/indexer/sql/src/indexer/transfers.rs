use {
    crate::{entity, error::Error},
    grug_types::{FlatCommitmentStatus, FlatEvent, FlatEventStatus, FlatEvtTransfer},
    indexer_sql::{block_to_index::MAX_ROWS_INSERT, entity as main_entity},
    itertools::Itertools,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait},
    std::collections::HashMap,
    uuid::Uuid,
};
#[cfg(feature = "metrics")]
use {
    metrics::{counter, describe_histogram, histogram},
    std::time::Instant,
};

pub(crate) async fn save_transfers(
    context: &crate::context::Context,
    block_height: u64,
) -> Result<(), Error> {
    #[cfg(feature = "tracing")]
    tracing::debug!("About to look at transfer events");

    #[cfg(feature = "metrics")]
    let start = Instant::now();

    let txn = context.db.begin().await?;

    // 1. get all successful transfers events from the database for this block
    let transfer_events: Vec<(FlatEvtTransfer, main_entity::events::Model)> =
        main_entity::events::Entity::find()
            .filter(main_entity::events::Column::Type.eq("transfer"))
            .filter(main_entity::events::Column::EventStatus.eq(FlatEventStatus::Ok.as_i16()))
            .filter(
                main_entity::events::Column::CommitmentStatus
                    .eq(FlatCommitmentStatus::Committed.as_i16()),
            )
            .filter(main_entity::events::Column::BlockHeight.eq(block_height))
            .all(&txn)
            .await?
            .into_iter()
            .flat_map(|te| {
                let flat_transfer_event: FlatEvent = serde_json::from_value(te.data.clone())?;

                if let FlatEvent::Transfer(flat_transfer_event) = flat_transfer_event {
                    Ok::<_, Error>((flat_transfer_event, te))
                } else {
                    #[cfg(feature = "tracing")]
                    tracing::error!(
                        "Wrong event type looking at transfers: {flat_transfer_event:?}"
                    );

                    Err(Error::WrongEventType)
                }
            })
            .collect::<Vec<_>>();

    #[cfg(feature = "metrics")]
    counter!("indexer.dango.hooks.transfer_events.total").increment(transfer_events.len() as u64);

    let transactions_by_id = main_entity::transactions::Entity::find()
        .filter(main_entity::transactions::Column::BlockHeight.eq(block_height as i64))
        .all(&txn)
        .await?
        .into_iter()
        .map(|t| (t.id, t))
        .collect::<HashMap<_, _>>();

    #[cfg(feature = "tracing")]
    tracing::info!(
        transfer_event_count = transfer_events.len(),
        "Looked at transfer events",
    );

    let mut idx = 0;

    // 2. create a transfer for each event
    let new_transfers: Vec<entity::transfers::ActiveModel> = transfer_events
        .into_iter()
        .flat_map(|(flat_transfer_event, te)| {
            flat_transfer_event
                .transfers
                .iter()
                .flat_map(|(recipient, coins)| {
                    #[cfg(feature = "tracing")]
                    if coins.is_empty() {
                        tracing::debug!(
                            "Transfer detected but coins is empty, won't create transfers",
                        );
                    }

                    coins
                        .into_iter()
                        .map(|coin| {
                            let res = entity::transfers::ActiveModel {
                                id: Set(Uuid::new_v4()),
                                idx: Set(idx),
                                block_height: Set(te.block_height),
                                tx_hash: Set(te
                                    .transaction_id
                                    .and_then(|tx_id| {
                                        transactions_by_id.get(&tx_id).map(|tx| tx.hash.clone())
                                    })
                                    .unwrap_or_default()),
                                created_at: Set(te.created_at),
                                from_address: Set(flat_transfer_event.sender.to_string()),
                                to_address: Set(recipient.to_string()),
                                amount: Set(coin.amount.to_string()),
                                denom: Set(coin.denom.to_string()),
                            };
                            idx += 1;
                            res
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect();

    #[cfg(feature = "metrics")]
    metrics::counter!("indexer.dango.hooks.transfers.total").increment(new_transfers.len() as u64);

    #[cfg(feature = "tracing")]
    tracing::debug!(
        new_transfers_count = new_transfers.len(),
        "Injecting new transfers",
    );

    if !new_transfers.is_empty() {
        // 3. insert the transfers into the database

        for transfers in new_transfers
            .into_iter()
            .chunks(MAX_ROWS_INSERT)
            .into_iter()
            .map(|c| c.collect())
            .collect::<Vec<Vec<_>>>()
        {
            entity::transfers::Entity::insert_many(transfers)
                .exec_without_returning(&txn)
                .await?;
        }
    }

    txn.commit().await?;

    #[cfg(feature = "tracing")]
    tracing::debug!("Injected new transfers");

    #[cfg(feature = "metrics")]
    histogram!("indexer.dango.hooks.transfers.duration").record(start.elapsed().as_secs_f64());

    Ok(())
}

#[cfg(feature = "metrics")]
pub fn init_metrics() {
    use metrics::describe_counter;

    describe_histogram!(
        "indexer.dango.hooks.transfers.duration",
        "Transfer hook duration in seconds"
    );

    describe_counter!(
        "indexer.dango.hooks.transfer_events.total",
        "Total transfer events processed"
    );

    describe_counter!(
        "indexer.dango.hooks.transfers.total",
        "Total transfers created"
    );

    describe_counter!(
        "indexer.dango.hooks.transfers.errors.total",
        "Total transfer hook errors"
    );
}
