#[cfg(feature = "async-graphql")]
use async_graphql::{Result, SimpleObject};
use {sea_orm::entity::prelude::*, serde::Deserialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Deserialize)]
#[sea_orm(table_name = "messages")]
#[cfg_attr(feature = "async-graphql", derive(SimpleObject))]
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
    pub created_at: DateTime,
    #[sea_orm(column_type = "JsonBinary")]
    pub data: Json,
    pub method_name: String,
    pub block_height: i64,
    pub contract_addr: Option<String>,
    pub sender_addr: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
