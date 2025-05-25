use {dango_types::account_factory, sea_orm::entity::prelude::*};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Hash)]
#[sea_orm(table_name = "accounts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub account_index: i32,
    #[sea_orm(unique)]
    pub address: String,
    pub account_type: account_factory::AccountType,
    pub created_at: DateTime,
    pub created_block_height: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::accounts_users::Entity",
        from = "Column::Id",
        to = "super::accounts_users::Column::AccountId"
    )]
    AccountUser,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        super::accounts_users::Relation::User.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::accounts_users::Relation::Account.def().rev())
    }
}

impl Related<super::accounts_users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AccountUser.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
