use {
    async_graphql::{ComplexObject, SimpleObject},
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
    pub event_status: i16,
    pub commitment_status: i16,
    pub data: serde_json::Value,
}

impl From<entity::events::Model> for Event {
    fn from(item: entity::events::Model) -> Self {
        Self {
            block_height: item.block_height as u64,
            created_at: Utc.from_utc_datetime(&item.created_at),
            event_idx: item.event_idx as u32,
            r#type: item.r#type.clone(),
            method: item.method,
            event_status: item.event_status,
            commitment_status: item.commitment_status,
            data: item.data,
        }
    }
}

#[ComplexObject]
impl Event {}
