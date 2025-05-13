use {
    crate::{
        entity::{self},
        error::Error,
    },
    async_trait::async_trait,
    dango_indexer_sql_migration::{Migrator, MigratorTrait},
    grug::Addr,
    grug_types::{FlatCommitmentStatus, FlatEvent, FlatEventStatus, FlatEvtTransfer},
    indexer_sql::{
        Context, block_to_index::BlockToIndex, entity as main_entity, hooks::Hooks as HooksTrait,
    },
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set},
    uuid::Uuid,
};

mod accounts;

#[derive(Clone)]
pub struct ContractAddrs {
    pub account_factory: Addr,
}

#[derive(Clone)]
pub struct Hooks {
    pub contract_addrs: ContractAddrs,
}

#[async_trait]
impl HooksTrait for Hooks {
    type Error = crate::error::Error;

    async fn start(&self, context: Context) -> Result<(), Self::Error> {
        Migrator::up(&context.db, None).await?;
        Ok(())
    }

    async fn post_indexing(
        &self,
        context: Context,
        block: BlockToIndex,
    ) -> Result<(), Self::Error> {
        self.save_transfers(&context, &block).await?;
        self.save_accounts(&context, &block).await?;

        Ok(())
    }
}

impl Hooks {
    async fn save_transfers(&self, context: &Context, block: &BlockToIndex) -> Result<(), Error> {
        #[cfg(feature = "tracing")]
        tracing::debug!("about to look at transfer events",);

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
                            "wrong event type looking at transfers: {:?}",
                            flat_transfer_event
                        );

                        Err(Error::WrongEventType)
                    }
                })
                .collect::<Vec<_>>();

        #[cfg(feature = "tracing")]
        tracing::debug!(
            transfer_event_count = transfer_events.len(),
            "looked at transfer events",
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
            "injecting new transfers",
        );

        if !new_transfers.is_empty() {
            // 3. insert the transfers into the database
            entity::transfers::Entity::insert_many(new_transfers)
                .exec_without_returning(&context.db)
                .await?;
        }

        #[cfg(feature = "tracing")]
        tracing::debug!("injected new transfers");

        Ok(())
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*, crate::entity, assertor::*, grug_app::Indexer, grug_types::MockStorage,
        indexer_sql::non_blocking_indexer::IndexerBuilder, sea_orm::EntityTrait,
    };

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn build_with_hooks() -> anyhow::Result<()> {
        let mut indexer = IndexerBuilder::default()
            .with_memory_database()
            .with_tmpdir()
            .with_hooks(Hooks {
                contract_addrs: ContractAddrs {
                    account_factory: Addr::mock(0),
                },
            })
            .build()?;

        let storage = MockStorage::new();

        assert!(!indexer.indexing);
        indexer.start(&storage).expect("Can't start Indexer");
        assert!(indexer.indexing);

        indexer.shutdown().expect("Can't shutdown Indexer");
        assert!(!indexer.indexing);

        let transfers = entity::transfers::Entity::find()
            .all(&indexer.context.db)
            .await?;
        assert_that!(transfers).is_empty();

        Ok(())
    }
}
