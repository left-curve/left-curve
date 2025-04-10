use sea_orm::entity::prelude::*;

#[derive(EnumIter, DeriveActiveEnum, Clone, Debug, PartialEq, Eq)]
#[sea_orm(rs_type = "i16", db_type = "Integer")]
pub enum AccountType {
    #[sea_orm(num_value = 0)]
    Spot,
    #[sea_orm(num_value = 1)]
    Margin,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "accounts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub username: String,
    pub index: i32,
    #[sea_orm(unique)]
    pub address: String,
    pub eth_address: Option<String>,
    pub account_type: AccountType,
    pub created_at: DateTime,
    pub created_block_height: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
