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
    grug_types::{Block, BlockOutcome, JsonSerExt, Tx, TxOutcome},
    sea_orm::{prelude::*, sqlx::types::chrono::NaiveDateTime, sqlx::types::chrono::TimeZone, Set},
};

#[derive(Debug, Default)]
pub struct Models {
    pub block: entity::blocks::ActiveModel,
    pub transactions: Vec<entity::transactions::ActiveModel>,
    pub messages: Vec<entity::messages::ActiveModel>,
    pub events: Vec<entity::events::ActiveModel>,
    pub next_event_id: u32,
}

impl Models {
    pub fn push(
        &mut self,
        block: &Block,
        tx: &Tx,
        tx_outcome: &TxOutcome,
    ) -> crate::error::Result<()> {
        let transaction_id = Uuid::new_v4();
        let sender = tx.sender.to_string();
        let new_transaction = entity::transactions::ActiveModel {
            id: Set(transaction_id),
            has_succeeded: Set(tx_outcome.result.is_ok()),
            error_message: Set(tx_outcome.clone().result.err()),
            gas_wanted: Set(tx.gas_limit.try_into()?),
            gas_used: Set(tx_outcome.gas_used.try_into()?),
            created_at: self.block.created_at.clone(),
            block_height: self.block.block_height.clone(),
            hash: Set("".to_string()),
            data: Set(tx.data.clone().into_inner()),
            sender: Set(sender.clone()),
            credential: Set(tx.credential.clone().into_inner()),
        };

        self.transactions.push(new_transaction);

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
                block_height: self.block.block_height.clone(),
                created_at: self.block.created_at.clone(),
                method_name: Set(method_name),
                data: Set(serialized_message.into_inner()),
                contract_addr: Set(contract_addr),
                sender_addr: Set(sender.clone()),
            };

            self.messages.push(new_message);
        }

        let epoch_millis = block.info.timestamp.into_millis();
        let seconds = (epoch_millis / 1_000) as i64;
        let nanoseconds = ((epoch_millis % 1_000) * 1_000_000) as u32;
        let naive_datetime = sea_orm::sqlx::types::chrono::Utc
            .timestamp_opt(seconds, nanoseconds)
            .single()
            .unwrap_or_default()
            .naive_utc();

        let (flatten_events, next_id) = flat_commitment_status(
            block.info.height,
            IndexCategory::Tx,
            0,
            self.next_event_id,
            tx_outcome.events.withhold.clone(),
        );
        for event in flatten_events {
            self.events.push(build_active_model(
                &event,
                &block,
                Some(transaction_id.clone()),
                naive_datetime,
            )?);
        }
        self.next_event_id = next_id;

        let (flatten_events, next_id) = flat_commitment_status(
            block.info.height,
            IndexCategory::Tx,
            0,
            self.next_event_id,
            tx_outcome.events.authenticate.clone(),
        );
        for event in flatten_events {
            self.events.push(build_active_model(
                &event,
                &block,
                Some(transaction_id),
                naive_datetime,
            )?);
        }
        self.next_event_id = next_id;

        let (flatten_events, next_id) = flat_commitment_status(
            block.info.height,
            IndexCategory::Tx,
            0,
            self.next_event_id,
            tx_outcome.events.msgs_and_backrun.clone(),
        );
        for event in flatten_events {
            self.events.push(build_active_model(
                &event,
                &block,
                Some(transaction_id.clone()),
                naive_datetime,
            )?);
        }
        self.next_event_id = next_id;

        let (flatten_events, next_id) = flat_commitment_status(
            block.info.height,
            IndexCategory::Tx,
            0,
            self.next_event_id,
            tx_outcome.events.finalize.clone(),
        );
        for event in flatten_events {
            self.events.push(build_active_model(
                &event,
                &block,
                Some(transaction_id.clone()),
                naive_datetime,
            )?);
        }
        self.next_event_id = next_id;

        Ok(())
    }

    pub fn build(block: &Block, block_outcome: &BlockOutcome) -> Result<Self, IndexerError> {
        let epoch_millis = block.info.timestamp.into_millis();
        let seconds = (epoch_millis / 1_000) as i64;
        let nanoseconds = ((epoch_millis % 1_000) * 1_000_000) as u32;

        let naive_datetime = sea_orm::sqlx::types::chrono::Utc
            .timestamp_opt(seconds, nanoseconds)
            .single()
            .unwrap_or_default()
            .naive_utc();

        let mut next_id = 1;
        let mut events = vec![];

        for cron_outcome in block_outcome.cron_outcomes.iter() {
            let (flatten_events, next_stored_id) = flat_commitment_status(
                block.info.height,
                IndexCategory::Cron,
                0,
                next_id,
                cron_outcome.cron_event.clone(),
            );

            for event in flatten_events {
                events.push(build_active_model(&event, &block, None, naive_datetime)?);
            }
            next_id = next_stored_id;
        }

        let block = entity::blocks::ActiveModel {
            id: Set(Uuid::new_v4()),
            block_height: Set(block.info.height.try_into()?),
            created_at: Set(naive_datetime),
            hash: Set(block.info.hash.to_string()),
            app_hash: Set(block_outcome.app_hash.to_string()),
        };

        Ok(Self {
            block,
            events,
            next_event_id: next_id,
            ..Default::default()
        })
    }
}

fn build_active_model(
    index_event: &IndexEvent,
    block: &Block,
    tx_id: Option<uuid::Uuid>,
    created_at: NaiveDateTime,
) -> crate::error::Result<entity::events::ActiveModel> {
    /*
        attributes: Object {
            "commitment_status": String("committed"),
            "event": Object {
                "guest": Object {
                    "contract": String("0x3b341adf8fbf728c725a03c5f08ffdf48cbaf602"),
                    "method": String("finalize_fee"),
                },
            },
            "event_status": String("ok"),
            "id": Object {
                "block": Number(1),
                "category": String("tx"),
                "category_index": Number(0),
                "event_index": Number(9),
            },
            "parent_id": Object {
                "block": Number(1),
                "category": String("tx"),
                "category_index": Number(0),
                "event_index": Number(8),
            },
        },
        block_height: 1,
    */

    let data = serde_json::to_value(&index_event.event)?;

    // NOTE: I'm manually removing the top hash, I could also #[serde(flatten)] on `IndexEvent`
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
                "Can't get the top hash",
            )))
        },
    };

    //let data = if let Json::Object(mut map) = data {
    //    map.keys()
    //        .next()
    //        .and_then(|key| map.remove(key))
    //        .unwrap_or_default()
    //} else {
    //    return Err(IndexerError::Anyhow(anyhow::anyhow!(
    //        "Can't get the top hash",
    //    )));
    //};

    let method = data
        .get("method")
        .and_then(|s| s.as_str())
        .map(|c| c.to_string());

    //attributes: Object {
    //    "guest": Object {
    //        "contract": String("0x3b341adf8fbf728c725a03c5f08ffdf48cbaf602"),
    //        "method": String("withhold_fee"),
    //    },
    //},

    let event_status = index_event.event_status.to_string();
    let commitment_status = index_event.commitment_status.to_string();

    Ok(entity::events::ActiveModel {
        id: Set(uuid::Uuid::new_v4()),
        transaction_id: Set(tx_id),
        created_at: Set(created_at),
        r#type: Set(index_event.event.to_string()),
        method: Set(method),
        attributes: Set(data),
        event_status: Set(event_status),
        commitment_status: Set(commitment_status),
        order_id: Set(index_event.id.event_index as i32),
        block_height: Set(block.info.height.try_into()?),
    })
}
