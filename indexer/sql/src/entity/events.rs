use {
    grug_types::{FlatCategory, FlatCommitmentStatus, FlatEventStatus},
    sea_orm::entity::prelude::*,
};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "commitment_status")]
pub enum CommitmentStatus {
    #[sea_orm(string_value = "Committed")]
    Committed,
    #[sea_orm(string_value = "Failed")]
    Failed,
    #[sea_orm(string_value = "Reverted")]
    Reverted,
}

impl From<FlatCommitmentStatus> for CommitmentStatus {
    fn from(value: FlatCommitmentStatus) -> Self {
        match value {
            FlatCommitmentStatus::Committed => CommitmentStatus::Committed,
            FlatCommitmentStatus::Failed => CommitmentStatus::Failed,
            FlatCommitmentStatus::Reverted => CommitmentStatus::Reverted,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "event_status")]
pub enum EventStatus {
    #[sea_orm(string_value = "Ok")]
    Ok,
    #[sea_orm(string_value = "Failed")]
    Failed,
    #[sea_orm(string_value = "NestedFailed")]
    NestedFailed,
    #[sea_orm(string_value = "Handled")]
    Handled,
}

impl From<FlatEventStatus> for EventStatus {
    fn from(value: FlatEventStatus) -> Self {
        match value {
            FlatEventStatus::Ok => EventStatus::Ok,
            FlatEventStatus::Failed(_) => EventStatus::Failed,
            FlatEventStatus::NestedFailed => EventStatus::NestedFailed,
            FlatEventStatus::Handled(_) => EventStatus::Handled,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "transaction_type")]
pub enum TransactionType {
    #[sea_orm(string_value = "Cron")]
    Cron,
    #[sea_orm(string_value = "Tx")]
    Tx,
}

impl From<FlatCategory> for TransactionType {
    fn from(value: FlatCategory) -> Self {
        match value {
            FlatCategory::Cron => TransactionType::Cron,
            FlatCategory::Tx => TransactionType::Tx,
        }
    }
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    DeriveEntityModel,
    Eq,
    /* Default,
     * serde :: Serialize,
     * serde :: Deserialize, */
)]
#[sea_orm(table_name = "events")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub transaction_id: Option<Uuid>,
    pub message_id: Option<Uuid>,
    pub created_at: DateTime,
    pub r#type: String,
    pub method: Option<String>,
    pub event_status: EventStatus,
    pub commitment_status: CommitmentStatus,
    pub transaction_type: i16,
    pub transaction_idx: i32,
    pub message_idx: Option<i32>,
    pub event_idx: i32,
    #[sea_orm(column_type = "JsonBinary")]
    pub data: Json,
    pub block_height: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
