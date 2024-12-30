use {
    crate::{
        entity,
        error::IndexerError,
        events::{
            flat_commitment_status, flat_tx_events, FlattenEvent, FlattenStatus, IndexCategory,
            IndexEvent,
        },
    },
    grug_math::Inner,
    grug_types::{Block, BlockOutcome, CommitmentStatus, JsonSerExt, Tx, TxOutcome},
    sea_orm::{
        prelude::*,
        sqlx::types::chrono::{NaiveDateTime, TimeZone},
        Set,
    },
};

#[derive(Debug, Default)]
pub struct Models {
    pub block: entity::blocks::ActiveModel,
    pub transactions: Vec<entity::transactions::ActiveModel>,
    pub messages: Vec<entity::messages::ActiveModel>,
    pub events: Vec<entity::events::ActiveModel>,
}

impl Models {
    pub fn build(block: &Block, block_outcome: &BlockOutcome) -> Result<Self, IndexerError> {
        let epoch_millis = block.info.timestamp.into_millis();
        let seconds = (epoch_millis / 1_000) as i64;
        let nanoseconds = ((epoch_millis % 1_000) * 1_000_000) as u32;

        let created_at = sea_orm::sqlx::types::chrono::Utc
            .timestamp_opt(seconds, nanoseconds)
            .single()
            .unwrap_or_default()
            .naive_utc();

        let mut next_id = 1;
        let mut events = vec![];

        // 1. Storing cron events
        {
            for cron_outcome in block_outcome.cron_outcomes.iter() {
                let (mut tx_events, new_next_id) = flatten_events(
                    &block,
                    IndexCategory::Cron,
                    1,
                    next_id,
                    cron_outcome.cron_event.clone(),
                    None,
                    created_at,
                )?;

                next_id = new_next_id;
                events.append(&mut tx_events);
            }
        }

        let mut transactions = vec![];
        let mut messages = vec![];

        // 2. Storing transactions, messages and events
        {
            let mut transaction_idx = 1;

            for (tx, tx_outcome) in block.txs.iter().zip(block_outcome.tx_outcomes.iter()) {
                let transaction_id = Uuid::new_v4();

                let sender = tx.sender.to_string();
                let new_transaction = entity::transactions::ActiveModel {
                    id: Set(transaction_id),
                    order_idx: Set(transaction_idx),
                    has_succeeded: Set(tx_outcome.result.is_ok()),
                    error_message: Set(tx_outcome.clone().result.err()),
                    gas_wanted: Set(tx.gas_limit.try_into()?),
                    gas_used: Set(tx_outcome.gas_used.try_into()?),
                    created_at: Set(created_at),
                    block_height: Set(block.info.height.try_into()?),
                    hash: Set("".to_string()),
                    data: Set(tx.data.clone().into_inner()),
                    sender: Set(sender.clone()),
                    credential: Set(tx.credential.clone().into_inner()),
                };

                transactions.push(new_transaction);

                // 3. Storing messages
                {
                    let mut message_idx = 1;
                    for message in tx.msgs.iter() {
                        let serialized_message = message.to_json_value()?;

                        let contract_addr = serialized_message
                            .get("contract")
                            .and_then(|c| c.as_str())
                            .map(|c| c.to_string());

                        let method_name = serialized_message
                            .as_object()
                            .and_then(|obj| obj.keys().next().cloned())
                            .unwrap_or_default();

                        let new_message = entity::messages::ActiveModel {
                            id: Set(Uuid::new_v4()),
                            transaction_id: Set(transaction_id),
                            order_idx: Set(message_idx),
                            block_height: Set(block.info.height.try_into()?),
                            created_at: Set(created_at),
                            method_name: Set(method_name),
                            data: Set(serialized_message.into_inner()),
                            contract_addr: Set(contract_addr),
                            sender_addr: Set(sender.clone()),
                        };

                        messages.push(new_message);
                        message_idx += 1;
                    }
                }

                // 4. Storing events
                {
                    let (mut tx_events, new_next_id) = flatten_events(
                        block,
                        IndexCategory::Tx,
                        1,
                        next_id,
                        tx_outcome.events.withhold.clone(),
                        Some(transaction_id),
                        created_at,
                    )?;

                    next_id = new_next_id;
                    events.append(&mut tx_events);

                    let (mut tx_events, new_next_id) = flatten_events(
                        block,
                        IndexCategory::Tx,
                        1,
                        next_id,
                        tx_outcome.events.authenticate.clone(),
                        Some(transaction_id),
                        created_at,
                    )?;

                    next_id = new_next_id;
                    events.append(&mut tx_events);

                    let (mut tx_events, new_next_id) = flatten_events(
                        block,
                        IndexCategory::Tx,
                        1,
                        next_id,
                        tx_outcome.events.msgs_and_backrun.clone(),
                        Some(transaction_id),
                        created_at,
                    )?;

                    next_id = new_next_id;
                    events.append(&mut tx_events);

                    let (mut tx_events, new_next_id) = flatten_events(
                        block,
                        IndexCategory::Tx,
                        1,
                        next_id,
                        tx_outcome.events.finalize.clone(),
                        Some(transaction_id),
                        created_at,
                    )?;

                    next_id = new_next_id;
                    events.append(&mut tx_events);
                }

                transaction_idx += 1;
            }
        }

        let block = entity::blocks::ActiveModel {
            id: Set(Uuid::new_v4()),
            block_height: Set(block.info.height.try_into()?),
            created_at: Set(created_at),
            hash: Set(block.info.hash.to_string()),
            app_hash: Set(block_outcome.app_hash.to_string()),
        };

        Ok(Self {
            block,
            events,
            transactions,
            messages,
        })
    }
}

fn flatten_events<T>(
    block: &Block,
    category: IndexCategory,
    category_id: u32,
    next_id: u32,
    commitment: CommitmentStatus<T>,
    transaction_id: Option<uuid::Uuid>,
    created_at: NaiveDateTime,
) -> crate::error::Result<(Vec<entity::events::ActiveModel>, u32)>
where
    T: FlattenStatus,
{
    let (flatten_events, next_id) = flat_commitment_status(
        block.info.height,
        category,
        category_id,
        next_id,
        commitment,
    );

    let mut events = vec![];
    for event in flatten_events {
        events.push(build_event_active_model(
            &event,
            block,
            transaction_id,
            created_at,
        )?);
    }

    Ok((events, next_id))
}

fn build_event_active_model(
    index_event: &IndexEvent,
    block: &Block,
    tx_id: Option<uuid::Uuid>,
    created_at: NaiveDateTime,
) -> crate::error::Result<entity::events::ActiveModel> {
    let data = serde_json::to_value(&index_event.event)?;

    // NOTE: I'm manually removing the top hash since it contains the same as `type`, I could also
    // use #[serde(flatten)] on `IndexEvent`
    let data = match data {
        Json::Object(map) => map
            .keys()
            .next()
            .and_then(|key| {
                let mut map = map.clone();
                map.remove(key)
            })
            .unwrap_or_default(),
        _ => {
            return Err(IndexerError::Anyhow(anyhow::anyhow!(
                "Can't get the top hash, never supposed to happen",
            )))
        },
    };

    let method = data
        .get("method")
        .and_then(|s| s.as_str())
        .map(|c| c.to_string());

    let event_status = index_event.event_status.to_string();
    let commitment_status = index_event.commitment_status.to_string();

    Ok(entity::events::ActiveModel {
        id: Set(uuid::Uuid::new_v4()),
        parent_id: Set(None),
        transaction_id: Set(tx_id),
        created_at: Set(created_at),
        r#type: Set(index_event.event.to_string()),
        method: Set(method),
        attributes: Set(data),
        event_status: Set(event_status),
        commitment_status: Set(commitment_status),
        order_idx: Set(index_event.id.event_index as i32),
        block_height: Set(block.info.height.try_into()?),
    })
}

macro_rules! flatten_and_append {
    ($block:expr, $category:expr, $category_id:expr, $next_id:expr, $commitment:expr, $tx_id:expr, $created_at:expr, $events:expr) => {{
        let (mut tx_events, new_next_id) = flatten_events(
            $block,
            $category,
            $category_id,
            $next_id,
            $commitment,
            $tx_id,
            $created_at,
        )?;
        $next_id = new_next_id;
        $events.append(&mut tx_events);
    }};
}
