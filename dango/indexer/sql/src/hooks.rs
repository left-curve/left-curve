use {
    crate::{
        entity::{self, accounts::AccountType},
        error::Error,
    },
    async_trait::async_trait,
    dango_indexer_sql_migration::{Migrator, MigratorTrait},
    dango_types::{
        account_factory::{self, AccountParams},
        auth::Key,
    },
    grug::{Addr, ByteArray, EventName, Inner, JsonDeExt, Op},
    grug_types::{FlatCommitmentStatus, FlatEvent, FlatEventStatus, FlatEvtTransfer, SearchEvent},
    indexer_sql::{
        Context, block_to_index::BlockToIndex, entity as main_entity, hooks::Hooks as HooksTrait,
    },
    itertools::Itertools,
    sea_orm::{
        ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait, sea_query,
        sqlx::types::chrono::TimeZone,
    },
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

        if detected_accounts.is_empty() {
            return Ok(());
        }

        tracing::info!("Detected accounts: {:?}", detected_accounts);

        let epoch_millis = block.block.info.timestamp.into_millis();
        let seconds = (epoch_millis / 1_000) as i64;
        let nanoseconds = ((epoch_millis % 1_000) * 1_000_000) as u32;

        let created_at = sea_orm::sqlx::types::chrono::Utc
            .timestamp_opt(seconds, nanoseconds)
            .single()
            .unwrap_or_default()
            .naive_utc();

        let accounts = detected_accounts
            .into_iter()
            .flat_map(|(_, account)| {
                let Some(account_type) = account.account_type else {
                    #[cfg(feature = "tracing")]
                    tracing::error!(
                        block_height = block.block.info.height,
                        "Found no account_type, shouldn't happen"
                    );
                    return None;
                };

                Some(entity::accounts::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    username: Set(account.username.into_inner()),
                    // TODO: get the index value
                    index: Set(0),
                    address: Set(account
                        .address
                        .map(|address| address.to_string())
                        .unwrap_or_default()),
                    eth_address: Set(account.eth_address.map(|address| address.to_string())),
                    account_type: Set(account_type),
                    created_block_height: Set(block.block.info.height as i64),
                    created_at: Set(created_at),
                })
            })
            .collect::<Vec<_>>();

        // I have to do with chunks to avoid psql errors with too many items
        let chunk_size = 1000;

        let txn = context.db.begin().await?;

        let chunked_accounts = accounts
            .into_iter()
            .chunks(chunk_size)
            .into_iter()
            .map(|chunk| chunk.collect())
            .collect::<Vec<Vec<_>>>();

        for accounts in chunked_accounts {
            let existing_accounts = entity::accounts::Entity::find()
                .filter(
                    entity::accounts::Column::Username
                        .is_in(
                            accounts
                                .iter()
                                .map(|a| a.username.as_ref())
                                .collect::<Vec<_>>(),
                        )
                        .or(entity::accounts::Column::Address.is_in(
                            accounts
                                .iter()
                                .map(|a| a.address.as_ref())
                                .collect::<Vec<_>>(),
                        )),
                )
                .all(&txn)
                .await?;

            let existing_accounts_by_username = existing_accounts
                .iter()
                .map(|a| (a.username.clone(), a))
                .collect::<HashMap<_, _>>();
            let existing_accounts_by_address = existing_accounts
                .iter()
                .map(|a| (a.address.clone(), a))
                .collect::<HashMap<_, _>>();

            entity::accounts::Entity::insert_many(accounts)
            // TODO: choose what I should do when there is a conflict, what to overwrite?
                // .on_conflict(
                //     sea_query::OnConflict::columns(vec![entity::accounts::Column::Address])
                //         .update_columns(vec![entity::accounts::Column::Username])
                //         .to_owned(),
                // )
                // .on_conflict(
                //     sea_query::OnConflict::columns(vec![entity::accounts::Column::Username])
                //         .update_columns(vec![entity::accounts::Column::Address])
                //         .to_owned(),
                // )
                .exec_without_returning(&txn)
                .await?;
        }

        txn.commit().await?;

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
