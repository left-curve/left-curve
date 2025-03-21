use {
    grug_types::{FlatCategory, FlatCommitmentStatus, FlatEventStatus},
    sea_orm::entity::prelude::*,
};

#[derive(EnumIter, DeriveActiveEnum, Clone, Debug, PartialEq, Eq)]
#[sea_orm(rs_type = "i16", db_type = "Integer")]
pub enum CommitmentStatus {
    #[sea_orm(num_value = 0)]
    Committed,
    #[sea_orm(num_value = 1)]
    Failed,
    #[sea_orm(num_value = 2)]
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

#[derive(EnumIter, DeriveActiveEnum, Clone, Debug, PartialEq, Eq)]
#[sea_orm(rs_type = "i16", db_type = "Integer")]
pub enum EventStatus {
    #[sea_orm(num_value = 0)]
    Ok,
    #[sea_orm(num_value = 1)]
    Failed,
    #[sea_orm(num_value = 2)]
    NestedFailed,
    #[sea_orm(num_value = 3)]
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

#[derive(EnumIter, DeriveActiveEnum, Clone, Debug, PartialEq, Eq)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum TransactionType {
    #[sea_orm(num_value = 0)]
    Cron,
    #[sea_orm(num_value = 1)]
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

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
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
    pub transaction_type: i32,
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
