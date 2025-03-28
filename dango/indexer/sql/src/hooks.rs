use {
    crate::{entity, error::Error},
    async_trait::async_trait,
    dango_indexer_sql_migration::{Migrator, MigratorTrait},
    dango_types::{
        account_factory::{self, AccountParams},
        auth::Key,
    },
    grug::{Addr, ByteArray, EventName, JsonDeExt, Op},
    grug_types::{FlatCommitmentStatus, FlatEvent, FlatEventStatus, FlatEvtTransfer, SearchEvent},
    indexer_sql::{
        Context, block_to_index::BlockToIndex, entity as main_entity, hooks::Hooks as HooksTrait,
    },
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set},
    std::collections::HashMap,
    uuid::Uuid,
};

#[derive(Clone)]
pub struct ContractAddrs {
    pub account_factory: Addr,
}

#[derive(Clone)]
pub struct Hooks {
    pub contract_addrs: ContractAddrs,
}

#[derive(Debug)]
struct AccountDetails {
    username: account_factory::Username,
    address: Option<Addr>,
    eth_address: Option<ByteArray<33>>,
    params: Option<AccountParams>,
    account_type: Option<AccountType>,
}

#[derive(Debug)]
enum AccountType {
    Spot,
    Margin,
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

        if !new_transfers.is_empty() {
            // 3. insert the transfers into the database
            entity::transfers::Entity::insert_many(new_transfers)
                .exec_without_returning(&context.db)
                .await?;
        }

        Ok(())
    }

    async fn save_accounts(&self, context: &Context, block: &BlockToIndex) -> Result<(), Error> {
        // Using code from https://github.com/left-curve/galxe-bot/blob/main/quest-1/src/quest.rs

        // TODO: when `events.method` is added to the event, use it to filter out events before going
        // through all events here (slower)

        let mut detected_accounts: HashMap<account_factory::Username, AccountDetails> =
            HashMap::new();

        for tx in block.block_outcome.tx_outcomes.iter() {
            if tx.result.is_err() {
                tracing::debug!("tx failed");
                continue;
            }

            let flat = tx.events.clone().flat();

            for event in flat {
                if event.commitment_status != FlatCommitmentStatus::Committed {
                    continue;
                }

                let FlatEvent::ContractEvent(event) = event.event else {
                    continue;
                };

                match event.ty.as_str() {
                    // Search for new user registration
                    account_factory::UserRegistered::EVENT_NAME => {
                        let Ok(event) = event
                            .data
                            .deserialize_json::<account_factory::UserRegistered>()
                        else {
                            continue;
                        };

                        // TODO: can the eth address be in the event data?

                        let _account = detected_accounts.entry(event.username.clone()).or_insert(
                            AccountDetails {
                                address: None,
                                username: event.username,
                                eth_address: None,
                                params: None,
                                account_type: None,
                            },
                        );
                    },
                    // Quest: Create a new account
                    account_factory::AccountRegistered::EVENT_NAME => {
                        let Ok(event) = event
                            .data
                            .deserialize_json::<account_factory::AccountRegistered>()
                        else {
                            continue;
                        };

                        if let AccountParams::Spot(params) | AccountParams::Margin(params) =
                            &event.params
                        {
                            let account = detected_accounts.entry(params.owner.clone()).or_insert(
                                AccountDetails {
                                    address: None,
                                    username: params.owner.clone(),
                                    eth_address: None,
                                    params: None,
                                    account_type: None,
                                },
                            );

                            if let AccountParams::Spot(_) = event.params {
                                account.account_type = Some(AccountType::Spot)
                            };
                            if let AccountParams::Margin(_) = event.params {
                                account.account_type = Some(AccountType::Margin)
                            };
                            account.address = Some(event.address);
                            account.params = Some(event.params);
                        }
                    },
                    // Detect Sepck256k1 key update
                    account_factory::KeyUpdated::EVENT_NAME => {
                        let Ok(event) =
                            event.data.deserialize_json::<account_factory::KeyUpdated>()
                        else {
                            continue;
                        };

                        if let Op::Insert(Key::Secp256k1(key)) = event.key {
                            let account = detected_accounts
                                .entry(event.username.clone())
                                .or_insert(AccountDetails {
                                    address: None,
                                    username: event.username,
                                    eth_address: None,
                                    params: None,
                                    account_type: None,
                                });

                            account.eth_address = Some(key);
                        }
                    },
                    _ => {},
                }
            }
        }

        tracing::info!("Detected accounts: {:?}", detected_accounts);

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
