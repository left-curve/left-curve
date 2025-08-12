use {
    crate::{entity, error::Result},
    grug_types::{
        Addr, Block, BlockOutcome, CommitmentStatus, EventId, Extractable, FlatCategory,
        FlatEventInfo, FlattenStatus, Inner, JsonSerExt, flatten_commitment_status,
    },
    sea_orm::{Set, TryIntoModel, prelude::*, sqlx::types::chrono::NaiveDateTime},
    std::{
        collections::{HashMap, HashSet},
        sync::Arc,
    },
};

#[derive(Debug, Default)]
pub struct Models {
    pub block: entity::blocks::ActiveModel,
    pub transactions: Vec<entity::transactions::ActiveModel>,
    pub messages: Vec<entity::messages::ActiveModel>,
    pub events: Vec<entity::events::ActiveModel>,
    pub events_by_address: HashMap<Addr, Vec<Arc<entity::events::Model>>>,
}

impl Models {
    pub fn build(block: &Block, block_outcome: &BlockOutcome) -> Result<Self> {
        let created_at = block.info.timestamp.to_naive_date_time();

        let mut event_id = EventId::new(block.info.height, FlatCategory::Cron, 0, 0);

        let mut transactions = vec![];
        let mut messages = vec![];
        let mut events = vec![];
        let mut events_by_address: HashMap<Addr, Vec<Arc<entity::events::Model>>> = HashMap::new();

        // 1. Storing cron events
        {
            for (cron_idx, cron_outcome) in block_outcome.cron_outcomes.iter().enumerate() {
                event_id.category_index = cron_idx as u32;

                let active_models = flatten_events(
                    block,
                    &mut events_by_address,
                    &mut event_id,
                    cron_outcome.cron_event.clone(),
                    None,
                    &[],
                    created_at,
                )?;

                events.extend(active_models);
            }
        }

        // 2. Storing transactions, messages and events
        {
            event_id.category = FlatCategory::Tx;

            for (transaction_idx, ((tx, tx_hash), tx_outcome)) in block
                .txs
                .iter()
                .zip(block_outcome.tx_outcomes.iter())
                .enumerate()
            {
                let transaction_id = Uuid::new_v4();

                let sender = tx.sender.to_string();
                let new_transaction = entity::transactions::ActiveModel {
                    id: Set(transaction_id),
                    transaction_idx: Set(transaction_idx as i32),
                    transaction_type: Set(FlatCategory::Tx),
                    has_succeeded: Set(tx_outcome.result.is_ok()),
                    error_message: Set(tx_outcome.clone().result.err()),
                    gas_wanted: Set(tx.gas_limit.try_into()?),
                    gas_used: Set(tx_outcome.gas_used.try_into()?),
                    created_at: Set(created_at),
                    block_height: Set(block.info.height.try_into()?),
                    hash: Set(tx_hash.to_string()),
                    data: Set(tx.data.clone().into_inner()),
                    sender: Set(sender.clone()),
                    credential: Set(tx.credential.clone().into_inner()),
                };

                transactions.push(new_transaction);

                event_id.category_index = transaction_idx as u32;

                let active_models = flatten_events(
                    block,
                    &mut events_by_address,
                    &mut event_id,
                    tx_outcome.events.withhold.clone(),
                    Some(transaction_id),
                    &[],
                    created_at,
                )?;

                events.extend(active_models);

                let active_models = flatten_events(
                    block,
                    &mut events_by_address,
                    &mut event_id,
                    tx_outcome.events.authenticate.clone(),
                    Some(transaction_id),
                    &[],
                    created_at,
                )?;

                events.extend(active_models);

                let mut message_ids = vec![];

                // 3. Storing messages
                {
                    for (message_idx, message) in tx.msgs.iter().enumerate() {
                        let serialized_message = message.to_json_value()?;

                        let contract_addr = serialized_message
                            .get("contract")
                            .and_then(|c| c.as_str())
                            .map(|c| c.to_string());

                        let method_name = serialized_message
                            .as_object()
                            .and_then(|obj| obj.keys().next().cloned())
                            .unwrap_or_default();

                        let message_id = Uuid::new_v4();
                        message_ids.push(message_id);

                        let new_message = entity::messages::ActiveModel {
                            id: Set(message_id),
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
                    let active_models = flatten_events(
                        block,
                        &mut events_by_address,
                        &mut event_id,
                        tx_outcome.events.msgs_and_backrun.clone(),
                        Some(transaction_id),
                        &message_ids,
                        created_at,
                    )?;

                    events.extend(active_models);

                    let active_models = flatten_events(
                        block,
                        &mut events_by_address,
                        &mut event_id,
                        tx_outcome.events.finalize.clone(),
                        Some(transaction_id),
                        &[],
                        created_at,
                    )?;

                    events.extend(active_models);
                }
            }
        }

        let block = entity::blocks::ActiveModel {
            id: Set(Uuid::new_v4()),
            block_height: Set(block.info.height.try_into()?),
            created_at: Set(created_at),
            hash: Set(block.info.hash.to_string()),
            app_hash: Set(block_outcome.app_hash.to_string()),
            transactions_count: Set(transactions.len() as i32),
        };

        Ok(Self {
            block,
            events,
            transactions,
            messages,
            events_by_address,
        })
    }
}

fn flatten_events<T>(
    block: &Block,
    events_by_address: &mut HashMap<Addr, Vec<Arc<entity::events::Model>>>,
    next_id: &mut EventId,
    commitment: CommitmentStatus<T>,
    transaction_id: Option<uuid::Uuid>,
    message_ids: &[uuid::Uuid],
    created_at: NaiveDateTime,
) -> Result<Vec<entity::events::ActiveModel>>
where
    T: FlattenStatus,
{
    let flatten_events = flatten_commitment_status(next_id, commitment);
    next_id.increment_idx(&flatten_events);

    let mut active_models = vec![];
    // Store previous events ids to link current event to optional parent uuid
    let mut events_ids = HashMap::new();

    for event in flatten_events {
        let db_event_id = uuid::Uuid::new_v4();

        let message_id = match event.id.message_index {
            Some(idx) => {
                let message_id = message_ids.get(idx as usize).cloned();
                if message_id.is_none() {
                    unreachable!(
                        "message_id is none for message_index: {:?} ids: {:?}",
                        next_id.message_index, message_ids
                    );
                }
                message_id
            },
            None => None,
        };

        // For now, we assume most events contains less than 10 addresses.
        // With an initial capacity of 10, we only need to 1 heap allocation for
        // the entire event.
        // If later we find that most events contain more than 10 addresses and
        // the reallocation has a noticeable performance impact, we can increase
        // the initial capacity.
        let mut addresses = HashSet::with_capacity(10);
        event.event.extract_addresses(&mut addresses);

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

        active_models.push(db_event.clone());

        let db_event = Arc::new(db_event.try_into_model()?);

        for addr in addresses {
            events_by_address
                .entry(addr)
                .or_default()
                .push(db_event.clone());
        }
    }

    Ok(active_models)
}

fn build_event_active_model(
    index_event: &FlatEventInfo,
    block: &Block,
    tx_id: Option<uuid::Uuid>,
    message_id: Option<uuid::Uuid>,
    event_id: uuid::Uuid,
    parent_event_id: Option<uuid::Uuid>,
    created_at: NaiveDateTime,
) -> Result<entity::events::ActiveModel> {
    // I'm serializing `FlattenEvent` to `serde_json::Value` and then manually
    // removing the top hash which is serialized to.
    // I could also use #[serde(flatten)] on `FlattenEvent`
    let data = serde_json::to_value(&index_event.event)?;
    // Removing the top hash
    let inside_data = match &data {
        Json::Object(map) => map
            .keys()
            .next()
            .and_then(|key| {
                let mut map = map.clone();
                map.remove(key)
            })
            .unwrap_or_default(),
        _ => {
            unreachable!("can't get the top hash! never supposed to happen");
        },
    };

    let method = inside_data
        .get("method")
        .and_then(|s| s.as_str())
        .map(|c| c.to_string());

    Ok(entity::events::ActiveModel {
        id: Set(event_id),
        parent_id: Set(parent_event_id),
        transaction_id: Set(tx_id),
        message_id: Set(message_id),
        created_at: Set(created_at),
        r#type: Set(index_event.event.to_string()),
        method: Set(method),
        data: Set(data),
        event_status: Set(index_event.event_status.clone().into()),
        commitment_status: Set(index_event.commitment_status),
        transaction_idx: Set(index_event.id.category_index as i32),
        transaction_type: Set(index_event.id.category as i32),
        message_idx: Set(index_event.id.message_index.map(|i| i as i32)),
        event_idx: Set(index_event.id.event_index as i32),
        block_height: Set(block.info.height.try_into()?),
    })
}
