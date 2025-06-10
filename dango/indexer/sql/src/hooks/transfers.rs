use {
    crate::{
        entity::{self},
        error::Error,
        hooks::Hooks,
    },
    grug_types::{FlatCommitmentStatus, FlatEvent, FlatEventStatus, FlatEvtTransfer},
    indexer_sql::{Context, block_to_index::BlockToIndex, entity as main_entity},
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set},
    uuid::Uuid,
};

impl Hooks {
    pub(crate) async fn save_transfers(
        &self,
        context: &Context,
        block: &BlockToIndex,
    ) -> Result<(), Error> {
        #[cfg(feature = "tracing")]
        tracing::debug!("About to look at transfer events");

        // 1. get all successful transfers events from the database for this block
        let transfer_events: Vec<(FlatEvtTransfer, main_entity::events::Model)> =
            main_entity::events::Entity::find()
                .filter(main_entity::events::Column::Type.eq("transfer"))
                .filter(main_entity::events::Column::EventStatus.eq(FlatEventStatus::Ok.as_i16()))
                .filter(
                    main_entity::events::Column::CommitmentStatus
                        .eq(FlatCommitmentStatus::Committed.as_i16()),
                )
                .filter(main_entity::events::Column::BlockHeight.eq(block.block.info.height))
                .all(&context.db)
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

        #[cfg(feature = "tracing")]
        tracing::debug!(
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
                        coins
                            .into_iter()
                            .map(|coin| {
                                let res = entity::transfers::ActiveModel {
                                    id: Set(Uuid::new_v4()),
                                    idx: Set(idx),
                                    block_height: Set(te.block_height),
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
                .exec_without_returning(&context.db)
                .await?;
        }

        #[cfg(feature = "tracing")]
        tracing::debug!("Injected new transfers");

        Ok(())
    }
}
