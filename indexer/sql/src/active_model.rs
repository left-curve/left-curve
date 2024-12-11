#![allow(unused)]

use {
    crate::{entity, error::IndexerError},
    grug_math::Inner,
    grug_types::{
        BlockInfo, CommitmentStatus, ContractEvent, EventStatus, EvtGuest, EvtWithhold, JsonSerExt,
        SubEvent, Tx, TxOutcome,
    },
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

        // TODO: handle structured events

        let mut event_id = 0;

        let events: Vec<entity::events::ActiveModel> = match tx_outcome.events.withhold {
            CommitmentStatus::Committed(event) => Self::create_withhold_event(event),
            CommitmentStatus::Failed { event, error } => {
                vec![]
            },
            CommitmentStatus::Reverted { event, revert_by } => {
                vec![]
            },
            CommitmentStatus::NotReached => {
                vec![]
            },
        };

        match tx_outcome.events.authenticate {
            CommitmentStatus::Committed(_) => todo!(),
            CommitmentStatus::Failed { event, error } => todo!(),
            CommitmentStatus::Reverted { event, revert_by } => todo!(),
            CommitmentStatus::NotReached => {},
        }

        match tx_outcome.events.msgs_and_backrun {
            CommitmentStatus::Committed(_) => todo!(),
            CommitmentStatus::Failed { event, error } => todo!(),
            CommitmentStatus::Reverted { event, revert_by } => todo!(),
            CommitmentStatus::NotReached => {},
        }

        match tx_outcome.events.finalize {
            CommitmentStatus::Committed(_) => todo!(),
            CommitmentStatus::Failed { event, error } => todo!(),
            CommitmentStatus::Reverted { event, revert_by } => todo!(),
            CommitmentStatus::NotReached => {},
        }

        Ok(())
    }

    fn create_withhold_event(event: EvtWithhold) -> Vec<entity::events::ActiveModel> {
        // 1. keep sender, gas_limit, taxman and remove guest_event
        let mut json_event = serde_json::json! {
            {
                "sender": event.sender,
                "gas_limit": event.gas_limit,
                "taxman": event.taxman,
            }
        };

        todo!()
    }

    fn create_evt_status_evt_guest_event(
        event: EventStatus<EvtGuest>,
        parent_event_id: uuid::Uuid,
    ) -> Vec<entity::events::ActiveModel> {
        let mut results = vec![];

        match event {
            EventStatus::Ok(event) => {
                //
                todo!()
            },
            EventStatus::NestedFailed(event) => todo!(),
            EventStatus::Failed { event, error } => todo!(),
            EventStatus::NotReached => {},
        }

        results
    }

    fn create_evt_status_sub_event(
        event: EventStatus<SubEvent>,
        parent_event_id: uuid::Uuid,
    ) -> Vec<entity::events::ActiveModel> {
        let mut results = vec![];

        match event {
            EventStatus::Ok(event) => {
                //
                todo!()
            },
            EventStatus::NestedFailed(event) => todo!(),
            EventStatus::Failed { event, error } => todo!(),
            EventStatus::NotReached => {},
        }

        results
    }

    fn create_evt_guest_event(
        event: EvtGuest,
        parent_event_id: uuid::Uuid,
    ) -> Vec<entity::events::ActiveModel> {
        let mut results = vec![];

        let mut json_event = serde_json::json! {
            {
                "contract": event.contract,
                "method": event.method,
                "contract_events": event.contract_events,
            }
        };

        let sub_events = event
            .sub_events
            .into_iter()
            .map(|sub_event| Self::create_evt_status_sub_event(sub_event, Default::default()))
            .collect::<Vec<_>>();

        results
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
