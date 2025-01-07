use {
    crate::{
        entity,
        error::IndexerError,
        events::{
            flat_commitment_status, flat_tx_events, EventId, FlattenEvent, FlattenStatus,
            IndexCategory, IndexEvent,
        },
    },
    grug_math::Inner,
    grug_types::{Block, BlockOutcome, CommitmentStatus, JsonSerExt, Op, Tx, TxOutcome},
    sea_orm::{
        prelude::*,
        sqlx::types::chrono::{NaiveDateTime, TimeZone},
        Set,
    },
    std::collections::HashMap,
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

        let mut event_next_id = 0;

        let mut transactions = vec![];
        let mut messages = vec![];
        let mut events = vec![];

        // 1. Storing cron events
        {
            for (cron_idx, cron_outcome) in block_outcome.cron_outcomes.iter().enumerate() {
                let mut active_models_with_event_idx = flatten_events(
                    block,
                    EventId::new(
                        block.info.height,
                        IndexCategory::Cron,
                        cron_idx as u32,
                        event_next_id,
                    ),
                    cron_outcome.cron_event.clone(),
                    None,
                    None,
                    created_at,
                )?;

                if active_models_with_event_idx.active_models.is_empty() {
                    continue;
                }

                event_next_id = active_models_with_event_idx.event_idx + 1;
                events.append(&mut active_models_with_event_idx.active_models);
            }
        }

        // 2. Storing transactions, messages and events
        {
            for (transaction_idx, (tx, tx_outcome)) in block
                .txs
                .iter()
                .zip(block_outcome.tx_outcomes.iter())
                .enumerate()
            {
                let transaction_id = Uuid::new_v4();

                let sender = tx.sender.to_string();
                let new_transaction = entity::transactions::ActiveModel {
                    id: Set(transaction_id),
                    order_idx: Set(transaction_idx as i32),
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

                let mut active_models_with_event_idx = flatten_events(
                    block,
                    EventId::new(
                        block.info.height,
                        IndexCategory::Tx,
                        transaction_idx as u32,
                        event_next_id,
                    ),
                    tx_outcome.events.withhold.clone(),
                    Some(transaction_id),
                    None,
                    created_at,
                )?;

                if !active_models_with_event_idx.active_models.is_empty() {
                    event_next_id = active_models_with_event_idx.event_idx + 1;
                    events.append(&mut active_models_with_event_idx.active_models);
                }

                let mut active_models_with_event_idx = flatten_events(
                    block,
                    EventId::new(
                        block.info.height,
                        IndexCategory::Tx,
                        transaction_idx as u32,
                        event_next_id,
                    ),
                    tx_outcome.events.authenticate.clone(),
                    Some(transaction_id),
                    None,
                    created_at,
                )?;

                if !active_models_with_event_idx.active_models.is_empty() {
                    event_next_id = active_models_with_event_idx.event_idx + 1;
                    events.append(&mut active_models_with_event_idx.active_models);
                }

                // 3. Storing messages
                {
                    // TODO: should zip with events
                    for (message_idx, message) in tx.msgs.iter().enumerate()
                    // .zip(tx_outcome.events.msgs_and_backrun.msgs.iter())
                    {
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
                            order_idx: Set(message_idx as i32),
                            block_height: Set(block.info.height.try_into()?),
                            created_at: Set(created_at),
                            method_name: Set(method_name),
                            data: Set(serialized_message.into_inner()),
                            contract_addr: Set(contract_addr),
                            sender_addr: Set(sender.clone()),
                        };

                        messages.push(new_message);
                    }
                }

                // 4. Storing events
                {
                    // iterate over messages and backrun messages
                    let mut active_models_with_event_idx = flatten_events(
                        block,
                        EventId::new(
                            block.info.height,
                            IndexCategory::Tx,
                            transaction_idx as u32,
                            event_next_id,
                        ),
                        // loop here
                        tx_outcome.events.msgs_and_backrun.clone(),
                        Some(transaction_id),
                        None,
                        created_at,
                    )?;

                    if !active_models_with_event_idx.active_models.is_empty() {
                        event_next_id = active_models_with_event_idx.event_idx + 1;
                        events.append(&mut active_models_with_event_idx.active_models);
                    }

                    let mut active_models_with_event_idx = flatten_events(
                        block,
                        EventId::new(
                            block.info.height,
                            IndexCategory::Tx,
                            transaction_idx as u32,
                            event_next_id,
                        ),
                        tx_outcome.events.finalize.clone(),
                        Some(transaction_id),
                        None,
                        created_at,
                    )?;

                    if !active_models_with_event_idx.active_models.is_empty() {
                        event_next_id = active_models_with_event_idx.event_idx + 1;
                        events.append(&mut active_models_with_event_idx.active_models);
                    }
                }
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

struct ActiveModelsWithEventIdx {
    active_models: Vec<entity::events::ActiveModel>,
    event_idx: u32,
}

fn flatten_events<T>(
    block: &Block,
    event_id: EventId,
    commitment: CommitmentStatus<T>,
    transaction_id: Option<uuid::Uuid>,
    message_id: Option<uuid::Uuid>,
    created_at: NaiveDateTime,
) -> crate::error::Result<ActiveModelsWithEventIdx>
where
    T: FlattenStatus,
{
    let (flatten_events, next_id) = flat_commitment_status(
        event_id.block,
        event_id.category,
        event_id.category_index,
        event_id.event_index,
        commitment,
    );

    let mut active_models = vec![];
    // Store previous events ids to link current event to optional parent id
    let mut events_ids = HashMap::new();

    for event in flatten_events {
        let db_event_id = uuid::Uuid::new_v4();

        events_ids.insert(event.id.event_index, db_event_id);

        let db_event = build_event_active_model(
            &event,
            block,
            transaction_id,
            message_id,
            db_event_id,
            events_ids.get(&event.parent_id.event_index).cloned(),
            created_at,
        )?;

        active_models.push(db_event);
    }

    Ok(ActiveModelsWithEventIdx {
        active_models,
        event_idx: next_id,
    })
}

fn build_event_active_model(
    index_event: &IndexEvent,
    block: &Block,
    tx_id: Option<uuid::Uuid>,
    message_id: Option<uuid::Uuid>,
    event_id: uuid::Uuid,
    parent_event_id: Option<uuid::Uuid>,
    created_at: NaiveDateTime,
) -> crate::error::Result<entity::events::ActiveModel> {
    // I'm serializing `FlattenEvent` to `serde_json::Value` and then manually
    // removing the top hash which is serialized to.
    // I could also use #[serde(flatten)] on `FlattenEvent`
    let data = serde_json::to_value(&index_event.event)?;
    // Removing the top hash
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
        id: Set(event_id),
        parent_id: Set(parent_event_id),
        transaction_id: Set(tx_id),
        message_id: Set(message_id),
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
