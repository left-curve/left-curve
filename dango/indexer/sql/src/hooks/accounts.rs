use {
    crate::{
        entity::{self},
        error::Error,
        hooks::Hooks,
    },
    dango_types::account_factory::{self, AccountParams},
    grug::{EventName, Inner, JsonDeExt},
    grug_types::{FlatCommitmentStatus, FlatEvent, SearchEvent},
    indexer_sql::{Context, block_to_index::BlockToIndex},
    sea_orm::{
        ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set,
        TransactionTrait, sqlx::types::chrono::TimeZone,
    },
    uuid::Uuid,
};

impl Hooks {
    pub(crate) async fn save_accounts(
        &self,
        context: &Context,
        block: &BlockToIndex,
    ) -> Result<(), Error> {
        let mut user_registered_events = Vec::new();
        let mut account_registered_events = Vec::new();
        let mut account_key_added_events = Vec::new();
        let mut account_key_removed_events = Vec::new();

        // NOTE:
        // The kind of operations which needs to be executed after are :
        // - UserRegistered: a username, key and key hash. We should create a user entry.
        // - AccountRegistered: an address, username, We should create an account entry.
        // - KeyOwned: a username, key and key hash. We should update the users entry with the new key.
        // - KeyDisowned: a username and key hash. We should delete that key hash attached to that user.

        for tx in block.block_outcome.tx_outcomes.iter() {
            if tx.result.is_err() {
                #[cfg(feature = "tracing")]
                tracing::debug!("tx failed, skipping");

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
                    account_factory::UserRegistered::EVENT_NAME => {
                        let Ok(event) = event
                            .data
                            .deserialize_json::<account_factory::UserRegistered>()
                        else {
                            continue;
                        };

                        user_registered_events.push(event.clone());
                    },
                    account_factory::AccountRegistered::EVENT_NAME => {
                        let Ok(event) = event
                            .data
                            .deserialize_json::<account_factory::AccountRegistered>()
                        else {
                            continue;
                        };

                        account_registered_events.push(event);
                    },
                    account_factory::KeyOwned::EVENT_NAME => {
                        let Ok(event) = event.data.deserialize_json::<account_factory::KeyOwned>()
                        else {
                            continue;
                        };

                        account_key_added_events.push(event);
                    },
                    account_factory::KeyDisowned::EVENT_NAME => {
                        let Ok(event) = event
                            .data
                            .deserialize_json::<account_factory::KeyDisowned>()
                        else {
                            continue;
                        };

                        account_key_removed_events.push(event);
                    },
                    _ => {},
                }
            }
        }

        // TODO: refactor this, used around multiple places
        let epoch_millis = block.block.info.timestamp.into_millis();
        let seconds = (epoch_millis / 1_000) as i64;
        let nanoseconds = ((epoch_millis % 1_000) * 1_000_000) as u32;

        let created_at = sea_orm::sqlx::types::chrono::Utc
            .timestamp_opt(seconds, nanoseconds)
            .single()
            .unwrap_or_default()
            .naive_utc();
        //

        let txn = context.db.begin().await?;
        // I have to do with chunks to avoid psql errors with too many items
        let chunk_size = 1000;

        if !user_registered_events.is_empty() {
            #[cfg(feature = "tracing")]
            tracing::info!("Detected user_registered_events: {user_registered_events:?}");

            for user_register_events in user_registered_events.chunks(chunk_size) {
                let new_users = user_register_events
                    .iter()
                    .map(|user_register_event| entity::users::ActiveModel {
                        id: Set(Uuid::new_v4()),
                        username: Set(user_register_event.username.to_string()),
                        created_at: Set(created_at),
                        created_block_height: Set(block.block.info.height as i64),
                    })
                    .collect::<Vec<_>>();

                let new_public_keys = user_register_events
                    .iter()
                    .map(|user_register_event| entity::public_keys::ActiveModel {
                        id: Set(Uuid::new_v4()),
                        username: Set(user_register_event.username.to_string()),
                        key_hash: Set(user_register_event.key_hash.to_string()),
                        public_key: Set(user_register_event.key.to_string()),
                        key_type: Set(user_register_event.key.ty()),
                        created_at: Set(created_at),
                        created_block_height: Set(block.block.info.height as i64),
                    })
                    .collect::<Vec<_>>();

                entity::users::Entity::insert_many(new_users)
                    .exec_without_returning(&txn)
                    .await?;
                entity::public_keys::Entity::insert_many(new_public_keys)
                    .exec_without_returning(&txn)
                    .await?;
            }
        }

        if !account_registered_events.is_empty() {
            #[cfg(feature = "tracing")]
            tracing::info!("Detected account_registered_events: {account_registered_events:?}");

            for account_registered_event in account_registered_events {
                let new_account_id = Uuid::new_v4();
                let new_account = entity::accounts::ActiveModel {
                    id: Set(new_account_id),
                    address: Set(account_registered_event.address.to_string()),
                    account_type: Set(account_registered_event.clone().params.ty()),
                    account_index: Set(account_registered_event.index as i32),
                    created_at: Set(created_at),
                    created_block_height: Set(block.block.info.height as i64),
                };

                entity::accounts::Entity::insert(new_account)
                    .exec_without_returning(&txn)
                    .await?;

                match account_registered_event.params {
                    AccountParams::Spot(params) | AccountParams::Margin(params) => {
                        let username = params.owner;

                        if let Some(user_id) = entity::users::Entity::find()
                            .column(entity::users::Column::Id)
                            .filter(entity::users::Column::Username.eq(username.inner()))
                            .one(&txn)
                            .await?
                            .map(|user| user.id)
                        {
                            let new_account_user = entity::accounts_users::ActiveModel {
                                id: Set(Uuid::new_v4()),
                                account_id: Set(new_account_id),
                                user_id: Set(user_id),
                            };

                            entity::accounts_users::Entity::insert(new_account_user)
                                .exec_without_returning(&txn)
                                .await?;
                        }
                    },
                    AccountParams::Multi(params) => {
                        for username in params.members.keys() {
                            if let Some(user_id) = entity::users::Entity::find()
                                .column(entity::users::Column::Id)
                                .filter(entity::users::Column::Username.eq(username.inner()))
                                .one(&txn)
                                .await?
                                .map(|user| user.id)
                            {
                                let new_account_user = entity::accounts_users::ActiveModel {
                                    id: Set(Uuid::new_v4()),
                                    account_id: Set(new_account_id),
                                    user_id: Set(user_id),
                                };

                                entity::accounts_users::Entity::insert(new_account_user)
                                    .exec_without_returning(&txn)
                                    .await?;
                            }
                        }
                    },
                }
            }
        }

        if !account_key_added_events.is_empty() {
            #[cfg(feature = "tracing")]
            tracing::info!("Detected account_key_added_events: {account_key_added_events:?}");

            for account_key_added_event in account_key_added_events {
                let model = entity::public_keys::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    username: Set(account_key_added_event.username.to_string()),
                    key_hash: Set(account_key_added_event.key_hash.to_string()),
                    public_key: Set(account_key_added_event.key.to_string()),
                    key_type: Set(account_key_added_event.key.ty()),
                    created_at: Set(created_at),
                    created_block_height: Set(block.block.info.height as i64),
                };

                model.insert(&txn).await?;
            }
        }

        if !account_key_removed_events.is_empty() {
            #[cfg(feature = "tracing")]
            tracing::info!("Detected `account_key_removed_events`: {account_key_removed_events:?}");

            for account_key_removed_event in account_key_removed_events {
                entity::public_keys::Entity::delete_many()
                    .filter(
                        entity::public_keys::Column::Username
                            .eq(account_key_removed_event.username.to_string())
                            .and(
                                entity::public_keys::Column::KeyHash
                                    .eq(account_key_removed_event.key_hash.to_string()),
                            ),
                    )
                    .exec(&txn)
                    .await?;
            }
        }

        txn.commit().await?;

        Ok(())
    }
}
