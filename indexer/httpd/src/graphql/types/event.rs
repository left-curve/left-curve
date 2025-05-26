use {
    async_graphql::*,
    chrono::{DateTime, TimeZone, Utc},
    indexer_sql::entity,
    serde::Deserialize,
};

#[derive(Clone, Debug, SimpleObject, Deserialize, Default, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
#[graphql(complex)]
#[serde(default)]
pub struct Event {
    pub block_height: u64,
    pub created_at: DateTime<Utc>,
    pub event_idx: u32,
    pub r#type: String,
    pub method: Option<String>,
    pub event_status: EventStatus,
    pub commitment_status: CommitmentStatus,
    pub data: serde_json::Value,
}

#[derive(Enum, Copy, Clone, Default, Eq, PartialEq, Debug, Deserialize, Hash)]
#[serde(rename_all = "UPPERCASE")]
pub enum EventStatus {
    #[default]
    Ok,
    Failed,
    NestedFailed,
    Handled,
}

impl From<entity::events::EventStatus> for EventStatus {
    fn from(status: entity::events::EventStatus) -> EventStatus {
        match status {
            entity::events::EventStatus::Ok => EventStatus::Ok,
            entity::events::EventStatus::Failed => EventStatus::Failed,
            entity::events::EventStatus::NestedFailed => EventStatus::NestedFailed,
            entity::events::EventStatus::Handled => EventStatus::Handled,
        }
    }
}

#[derive(Enum, Copy, Clone, Default, Eq, PartialEq, Debug, Deserialize, Hash)]
#[serde(rename_all = "UPPERCASE")]
pub enum CommitmentStatus {
    #[default]
    Committed,
    Failed,
    Reverted,
}

impl From<entity::events::CommitmentStatus> for CommitmentStatus {
    fn from(status: entity::events::CommitmentStatus) -> CommitmentStatus {
        match status {
            entity::events::CommitmentStatus::Committed => CommitmentStatus::Committed,
            entity::events::CommitmentStatus::Failed => CommitmentStatus::Failed,
            entity::events::CommitmentStatus::Reverted => CommitmentStatus::Reverted,
        }
    }
}

impl From<entity::events::Model> for Event {
    fn from(item: entity::events::Model) -> Self {
        Self {
            block_height: item.block_height as u64,
            created_at: Utc.from_utc_datetime(&item.created_at),
            event_idx: item.event_idx as u32,
            r#type: item.r#type,
            method: item.method,
            event_status: item.event_status.into(),
            commitment_status: todo!(), // item.commitment_status.into(),
            data: item.data,
        }
    }
}

#[ComplexObject]
impl Event {}
