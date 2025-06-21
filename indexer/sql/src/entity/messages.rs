#[cfg(feature = "async-graphql")]
use async_graphql::{ComplexObject, Result, SimpleObject};
use {sea_orm::entity::prelude::*, serde::Deserialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Deserialize)]
#[sea_orm(table_name = "messages")]
#[cfg_attr(feature = "async-graphql", derive(SimpleObject))]
#[cfg_attr(feature = "async-graphql", graphql(complex))]
#[cfg_attr(feature = "async-graphql", graphql(name = "Message"))]
#[cfg_attr(feature = "async-graphql", serde(rename_all = "camelCase"))]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[cfg_attr(
        all(feature = "async-graphql", not(feature = "testing")),
        graphql(skip)
    )]
    pub id: Uuid,
    #[cfg_attr(
        all(feature = "async-graphql", not(feature = "testing")),
        graphql(skip)
    )]
    pub transaction_id: Uuid,
    pub order_idx: i32,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "crate::serde_iso8601")]
    pub created_at: DateTime,
    #[sea_orm(column_type = "JsonBinary")]
    pub data: Json,
    pub method_name: String,
    pub block_height: i64,
    pub contract_addr: Option<String>,
    pub sender_addr: String,
}

#[cfg(feature = "async-graphql")]
#[ComplexObject]
impl Model {
    /// Returns the creation timestamp in ISO8601 format with timezone
    async fn created_at(&self) -> String {
        let ts = grug_types::Timestamp::from_nanos(
            self.created_at.and_utc().timestamp_nanos_opt().unwrap_or(0) as u128,
        );
        ts.to_rfc3339_string()
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
