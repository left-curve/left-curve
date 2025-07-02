use {
    crate::{entity, error::Error, hooks::Indexer},
    grug_types::{FlatCommitmentStatus, FlatEvent, FlatEventStatus, FlatEvtTransfer},
    indexer_sql::entity as main_entity,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait},
    std::collections::HashMap,
    uuid::Uuid,
};
#[cfg(feature = "metrics")]
use {
    metrics::{describe_histogram, histogram},
    std::time::Instant,
};

impl Indexer {
    pub(crate) async fn save_transfers(&self, block_height: u64) -> Result<(), Error> {
        #[cfg(feature = "tracing")]
        tracing::debug!("About to look at transfer events");

        #[cfg(feature = "metrics")]
        let start = Instant::now();

        let txn = self.context.db.begin().await?;

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
                            tracing::warn!(
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

        #[cfg(feature = "tracing")]
        tracing::debug!(
            new_transfers_count = new_transfers.len(),
            "Injecting new transfers",
        );

        if !new_transfers.is_empty() {
            // 3. insert the transfers into the database
            entity::transfers::Entity::insert_many(new_transfers)
                .exec_without_returning(&txn)
                .await?;
        }

        txn.commit().await?;

        #[cfg(feature = "tracing")]
        tracing::debug!("Injected new transfers");

        #[cfg(feature = "metrics")]
        histogram!(
            "indexer.dango.hooks.transfers.duration",
            "block_height" => block_height.to_string()
        )
        .record(start.elapsed().as_secs_f64());

        Ok(())
    }
}

#[cfg(feature = "metrics")]
pub fn init_metrics() {
    describe_histogram!(
        "indexer.dango.hooks.transfers.duration",
        "Transfer hook duration in seconds"
    );
}
