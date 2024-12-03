use {
    crate::{entity, error::IndexerError},
    grug_math::Inner,
    grug_types::{BlockInfo, JsonSerExt, Tx, TxOutcome},
    sea_orm::{prelude::*, sqlx::types::chrono::TimeZone, Set},
};

#[derive(Debug, Default)]
pub struct Models {
    pub block: entity::blocks::ActiveModel,
    pub transactions: Vec<entity::transactions::ActiveModel>,
    pub messages: Vec<entity::messages::ActiveModel>,
    pub events: Vec<entity::events::ActiveModel>,
}

impl Models {
    pub fn push(&mut self, tx: Tx, tx_outcome: TxOutcome) -> crate::error::Result<()> {
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

        for event in tx_outcome.events.iter() {
            let serialized_attributes = event.attributes.to_json_value()?;

            let new_event = entity::events::ActiveModel {
                id: Set(Uuid::new_v4()),
                transaction_id: Set(transaction_id),
                block_height: self.block.block_height.clone(),
                created_at: self.block.created_at.clone(),
                r#type: Set(event.r#type.clone()),
                attributes: Set(serialized_attributes.into_inner()),
            };

            self.events.push(new_event);
        }

        Ok(())
    }

    pub fn build(block: &BlockInfo) -> Result<Self, IndexerError> {
        let epoch_millis = block.timestamp.into_millis();
        let seconds = (epoch_millis / 1_000) as i64;
        let nanoseconds = ((epoch_millis % 1_000) * 1_000_000) as u32;

        let naive_datetime = sea_orm::sqlx::types::chrono::Utc
            .timestamp_opt(seconds, nanoseconds)
            .single()
            .unwrap_or_default()
            .naive_utc();

        let block = entity::blocks::ActiveModel {
            id: Set(Uuid::new_v4()),
            block_height: Set(block.height.try_into()?),
            created_at: Set(naive_datetime),
            hash: Set(block.hash.to_string()),
        };

        Ok(Self {
            block,
            ..Default::default()
        })
    }
}
