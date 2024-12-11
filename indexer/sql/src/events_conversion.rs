#![allow(unused)]

use {
    crate::{entity, error::IndexerError},
    grug_math::Inner,
    grug_types::{
        self, ContractEvent, EvtGuest, EvtWithhold, Json, JsonSerExt, StdResult, SubEvent, TxEvents,
    },
    tracing::event,
};

#[derive(Debug, strum_macros::Display, serde::Serialize, serde::Deserialize, Clone)]
enum CommitmentStatus {
    Committed,
    Failed,
    Reverted,
    // Won't store non-existing events
    // NotReached,
}

#[derive(Debug, strum_macros::Display, serde::Serialize, serde::Deserialize, Clone)]
enum EventStatus {
    Ok,
    NestedFailed,
    Failed,
    // Won't store non-existing events
    // NotReached,
}

#[derive(Debug, strum_macros::Display, serde::Serialize, serde::Deserialize, Clone)]
enum HandleEventStatus {
    Ok,
    NestedFailed,
    Failed,
    Handled,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
/// This represent the event I will save on DB in a single row
struct Event {
    id: uuid::Uuid,
    parent_id: Option<uuid::Uuid>,
    commitment_status: Option<CommitmentStatus>,
    event_status: Option<EventStatus>,
    wasm_export_method: Option<String>, // EvtGuest.method
    r#type: Option<String>,             // ContractEvent.type
    data: serde_json::Value,
}

// EventId,
// TransactionId,
// Type,
// CommitmentStatus,
// Status,
// ParentId,
// Error,
// RevertBy,
// ContractAddr,
// Data,
// BlockHeight,
// CreatedAt,

trait GrugEventToActiveModels {
    fn to_active_models(&self, initial_event_id: usize) -> Vec<entity::events::ActiveModel>;
}

impl GrugEventToActiveModels for TxEvents {
    fn to_active_models(&self, mut initial_event_id: usize) -> Vec<entity::events::ActiveModel> {
        let mut results = vec![];

        let events: Vec<_> = match &self.withhold {
            grug_types::CommitmentStatus::Committed(event) => {
                create_withhold_event(CommitmentStatus::Committed, event.clone())
            },
            grug_types::CommitmentStatus::Failed { event, error } => {
                vec![]
            },
            grug_types::CommitmentStatus::Reverted { event, revert_by } => {
                vec![]
            },
            grug_types::CommitmentStatus::NotReached => {
                vec![]
            },
        };

        results
    }
}

fn create_withhold_event(commitment_status: CommitmentStatus, event: EvtWithhold) -> Vec<Event> {
    // keep sender, gas_limit, taxman and manually manage guest_event since it
    // includes other events
    let mut json_event = serde_json::json! {
        {
            "sender": event.sender,
            "gas_limit": event.gas_limit,
            "taxman": event.taxman,
        }
    };

    match event.guest_event {
        grug_types::EventStatus::Ok(event) => {
            create_guest_event(commitment_status, EventStatus::Ok, event);
            todo!()
        },
        grug_types::EventStatus::NestedFailed(event) => todo!(),
        grug_types::EventStatus::Failed { event, error } => todo!(),
        grug_types::EventStatus::NotReached => {
            todo!()
        },
    }

    todo!()
}

fn create_guest_event(
    commitment_status: CommitmentStatus,
    event_status: EventStatus,
    event: EvtGuest,
) -> Vec<Event> {
    let mut results: Vec<Event> = vec![];

    let mut json_event = serde_json::json! {
        {
            "contract": event.contract,
            "wasm_export_method": event.method,
        }
    };

    for contract_event in event.contract_events.clone() {
        let mut new_event = create_contract_event(
            commitment_status.clone(),
            event_status.clone(),
            event.clone(),
            contract_event,
        );

        results.push(new_event);
    }

    for sub_event_status in event.sub_events {
        results.push(create_sub_event_status(
            commitment_status.clone(),
            sub_event_status,
        ));
    }

    results
}

fn create_sub_event_status(
    commitment_status: CommitmentStatus,
    event: grug_types::EventStatus<SubEvent>,
) -> Event {
    match event {
        grug_types::EventStatus::Ok(event) => {
            // create_sub_event(commitment_status, EventStatus::Ok, event)
        },
        grug_types::EventStatus::NestedFailed(event) => todo!(),
        grug_types::EventStatus::Failed { event, error } => todo!(),
        grug_types::EventStatus::NotReached => {
            todo!()
        },
    }
    todo!()
}

fn create_event_status_event_guest(
    commitment_status: CommitmentStatus,
    event: grug_types::EventStatus<EvtGuest>,
) -> Event {
    match event {
        grug_types::EventStatus::Ok(event) => {
            create_guest_event(commitment_status, EventStatus::Ok, event);
            todo!()
        },
        grug_types::EventStatus::NestedFailed(event) => {
            create_guest_event(commitment_status, EventStatus::NestedFailed, event);
            todo!()
        },
        grug_types::EventStatus::Failed { event, error } => todo!(),
        grug_types::EventStatus::NotReached => {
            todo!()
        },
    }
    todo!()
}

fn create_sub_event(commitment_status: CommitmentStatus, event: &SubEvent) -> Event {
    todo!()
}

fn create_sub_event_handle_status(
    commitment_status: CommitmentStatus,
    handle_event: &grug_types::HandleEventStatus,
) -> Event {
    match handle_event {
        grug_types::HandleEventStatus::Ok(event) => {
            // create_sub_event(commitment_status, EventStatus::Ok, event)
            todo!()
        },
        grug_types::HandleEventStatus::NestedFailed(event) => todo!(),
        grug_types::HandleEventStatus::Failed { event, error } => todo!(),
        grug_types::HandleEventStatus::Handled { event, error } => {
            todo!()
        },
    }
    todo!()
}

fn create_contract_event(
    commitment_status: CommitmentStatus,
    event_status: EventStatus,
    event: EvtGuest,
    contract_event: ContractEvent,
) -> Event {
    let mut json_event = serde_json::json! {
        {
            "contract": event.contract,
            "wasm_export_method": event.method,
        }
    };

    Event {
        id: uuid::Uuid::new_v4(),
        parent_id: None,
        commitment_status: Some(commitment_status),
        event_status: Some(event_status),
        wasm_export_method: Some(event.method),
        r#type: Some(contract_event.ty),
        data: contract_event.data.into_inner(),
    }
}

fn create_event(
    commitment_status: CommitmentStatus,
    event_status: EventStatus,
    event: &grug_types::Event,
) -> crate::error::Result<Vec<Event>> {
    match event {
        grug_types::Event::Configure(event) => {
            let data = event.to_json_value()?;
            todo!()
        },
        grug_types::Event::Upload(event) => {
            let data = event.to_json_value()?;
            todo!()
        },
        _ => {
            todo!()
        },
    }
}

fn create_event_cron(
    commitment_status: CommitmentStatus,
    event_status: EventStatus,
    event_cron: grug_types::EvtCron,
) -> Vec<Event> {
    let mut json_event = serde_json::json! {
        {
            "contract": event_cron.contract,
            "time": event_cron.time,
            "next": event_cron.next,
        }
    };

    let events: Vec<Event> = match event_cron.guest_event {
        grug_types::EventStatus::Ok(event) => {
            create_guest_event(commitment_status, EventStatus::Ok, event);
            todo!()
        },
        grug_types::EventStatus::NestedFailed(event) => {
            create_guest_event(commitment_status, EventStatus::Failed, event);
            todo!()
        },
        grug_types::EventStatus::Failed { event, error } => todo!(),
        grug_types::EventStatus::NotReached => {
            vec![]
        },
    };

    todo!()
}
