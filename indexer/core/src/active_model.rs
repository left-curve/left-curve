use grug_math::Inner;
use grug_types::{BlockInfo, BlockOutcome, Message, Tx, TxOutcome};
use sea_orm::prelude::*;
use sea_orm::sqlx::types::chrono::TimeZone;
use sea_orm::Set;

#[derive(Debug, Default)]
pub struct Models {
    pub block: indexer_entity::blocks::ActiveModel,
    pub transactions: Vec<indexer_entity::transactions::ActiveModel>,
    pub messages: Vec<indexer_entity::messages::ActiveModel>,
    pub events: Vec<indexer_entity::events::ActiveModel>,
}

impl Models {
    pub fn push(&mut self, tx: &Tx, tx_outcome: &TxOutcome) {
        let transaction_id = Uuid::new_v4();
        let sender = tx.sender.to_string();
        let new_transaction = indexer_entity::transactions::ActiveModel {
            id: Set(transaction_id),
            has_succeeded: Set(tx_outcome.result.is_ok()),
            error_message: Set(tx_outcome
                .clone()
                .result
                .map_or_else(|err| Some(err), |_| None)),
            gas_wanted: Set(tx.gas_limit.try_into().unwrap()),
            gas_used: Set(tx_outcome.gas_used.try_into().unwrap()),
            created_at: self.block.created_at.clone(),
            block_height: self.block.block_height.clone(),
            hash: Set("".to_string()),
            data: Set(tx.data.clone().into_inner()),
            sender: Set(sender),
            credential: Set(tx.credential.clone().into_inner()),
        };
        self.transactions.push(new_transaction);

        for message in tx.msgs.iter() {
            let serialized_message = serde_json::to_value(message).unwrap();
            let contract_addr = serialized_message
                .get("contract")
                .and_then(|c| c.as_str())
                .map(|c| c.to_string());
            let method_name = serialized_message
                .as_object()
                .and_then(|obj| obj.keys().next().cloned())
                .unwrap_or_default();

            let new_message = indexer_entity::messages::ActiveModel {
                id: Set(Uuid::new_v4()),
                transaction_id: Set(transaction_id),
                block_height: self.block.block_height.clone(),
                created_at: self.block.created_at.clone(),
                method_name: Set(method_name),
                data: Set(serialized_message),
                addr: Set(contract_addr),
            };
            self.messages.push(new_message);
        }

        for event in tx_outcome.events.iter() {
            let serialized_attributes = serde_json::to_value(&event.attributes).unwrap();
            let new_event = indexer_entity::events::ActiveModel {
                id: Set(Uuid::new_v4()),
                transaction_id: Set(transaction_id),
                block_height: self.block.block_height.clone(),
                created_at: self.block.created_at.clone(),
                r#type: Set(event.r#type.clone()),
                attributes: Set(serialized_attributes),
            };
            self.events.push(new_event);
        }
    }

    pub fn build(block: &BlockInfo) -> Self {
        let epoch_millis = block.timestamp.into_millis();
        let seconds = (epoch_millis / 1_000) as i64;
        let nanoseconds = ((epoch_millis % 1_000) * 1_000_000) as u32;

        let naive_datetime = sea_orm::sqlx::types::chrono::Utc
            .timestamp_opt(seconds, nanoseconds)
            .single()
            .unwrap_or_default()
            .naive_utc();

        let block = indexer_entity::blocks::ActiveModel {
            id: Set(Uuid::new_v4()),
            block_height: Set(block.height.try_into().unwrap()),
            created_at: Set(naive_datetime),
            hash: Set(block.hash.to_string()),
        };

        Self {
            block,
            ..Default::default()
        }
    }
}
