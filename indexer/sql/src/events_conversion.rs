#![allow(unused)]

use {
    crate::entity,
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
    event_id: usize,
    parent_id: Option<usize>,
    commitment_status: CommitmentStatus,
    event_status: EventStatus,
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
//

trait GrugEventToActiveModels {
    fn to_active_models(&self, initial_event_id: usize) -> Vec<entity::events::ActiveModel>;
}

impl GrugEventToActiveModels for TxEvents {
    fn to_active_models(&self, mut initial_event_id: usize) -> Vec<entity::events::ActiveModel> {
        let mut results = vec![];

        let events: Vec<_> = match &self.withhold {
            grug_types::CommitmentStatus::Committed(event) => {
                create_withhold_event(CommitmentStatus::Committed, event)
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

fn create_withhold_event(commitment_status: CommitmentStatus, event: &EvtWithhold) -> Vec<Event> {
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
            create_guest_event(commitment_status, EventStatus::Ok, &event)
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
    event: &EvtGuest,
) -> Vec<Event> {
    let mut results: Vec<Event> = vec![];

    let mut json_event = serde_json::json! {
        {
            "contract": event.contract,
            "wasm_export_method": event.method,
        }
    };

    for contract_event in event.contract_events.iter() {
        let mut new_event = create_contract_event(
            commitment_status.clone(),
            event_status.clone(),
            event,
            contract_event,
        );

        results.push(new_event);
    }

    for sub_event_status in event.sub_events.iter() {
        results.push(create_sub_event_status(
            commitment_status.clone(),
            sub_event_status,
        ));
    }

    results
}

fn create_sub_event_status(
    commitment_status: CommitmentStatus,
    event: &grug_types::EventStatus<SubEvent>,
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
    event: &EvtGuest,
    contract_event: &ContractEvent,
) -> Event {
    let mut json_event = serde_json::json! {
        {
            "contract": event.contract,
            "wasm_export_method": event.method,
        }
    };

    Event {
        event_id: 0,
        parent_id: None,
        commitment_status,
        event_status,
        wasm_export_method: Some(event.method.clone()),
        r#type: Some(contract_event.ty.clone()),
        data: contract_event.data.clone().into_inner(),
    }
}

fn create_event(
    commitment_status: CommitmentStatus,
    event_status: EventStatus,
    event: &grug_types::Event,
) -> StdResult<Json> {
    match event {
        grug_types::Event::Configure(event) => Ok(event.to_json_value()?),
        grug_types::Event::Upload(event) => Ok(event.to_json_value()?),
        _ => {
            todo!()
        },
    }
}
