use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Hash)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub username: String,
    pub created_at: DateTime,
    pub created_block_height: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::accounts_users::Entity",
        from = "Column::Id",
        to = "super::accounts_users::Column::UserId"
    )]
    AccountUser,
    #[sea_orm(
        has_many = "super::public_keys::Entity",
        from = "Column::Username",
        to = "super::public_keys::Column::Username"
    )]
    PublicKeys,
}

impl Related<super::accounts_users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AccountUser.def()
    }
}

impl Related<super::public_keys::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PublicKeys.def()
    }
}

impl Related<super::accounts::Entity> for Entity {
    fn to() -> RelationDef {
        super::accounts_users::Relation::Account.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::accounts_users::Relation::User.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
