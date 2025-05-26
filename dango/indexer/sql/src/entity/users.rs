#[cfg(feature = "async-graphql")]
use async_graphql::{ComplexObject, Context, Result, SimpleObject};
use {
    sea_orm::entity::prelude::*,
    serde::{Deserialize, Serialize},
};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Hash, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
#[cfg_attr(feature = "async-graphql", derive(SimpleObject))]
#[cfg_attr(feature = "async-graphql", graphql(complex))]
#[cfg_attr(feature = "async-graphql", graphql(name = "User"))]
#[cfg_attr(feature = "async-graphql", serde(rename_all = "camelCase"))]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[cfg_attr(
        all(feature = "async-graphql", not(feature = "testing")),
        graphql(skip)
    )]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub username: String,
    pub created_at: DateTime,
    pub created_block_height: i64,
}

#[cfg(feature = "async-graphql")]
#[ComplexObject]
impl Model {
    pub async fn public_keys(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<crate::entity::public_keys::Model>> {
        let db = ctx.data::<DatabaseConnection>()?;

        Ok(self
            .find_related(crate::entity::public_keys::Entity)
            .all(db)
            .await?)

        // TODO: leaving this here for now, in case the relation doesn't work

        // let public_keys = entity::public_keys::Entity::find()
        //     .filter(entity::public_keys::Column::Username.eq(&self.username))
        //     .all(&app_ctx.db)
        //     .await?
        //     .into_iter()
        //     .map(PublicKey::from)
        //     .collect::<Vec<_>>();

        // Ok(public_keys)
    }

    pub async fn accounts(&self, ctx: &Context<'_>) -> Result<Vec<crate::entity::accounts::Model>> {
        let db = ctx.data::<DatabaseConnection>()?;

        Ok(self
            .find_related(crate::entity::accounts::Entity)
            .all(db)
            .await?)

        // TODO: leaving this here for now, in case the relation doesn't work

        // let account_ids = entity::accounts_users::Entity::find()
        //     .filter(entity::accounts_users::Column::UserId.eq(self.username.clone()))
        //     .all(&app_ctx.db)
        //     .await?
        //     .into_iter()
        //     .map(|item| item.account_id)
        //     .collect::<Vec<_>>();

        // let accounts = entity::accounts::Entity::find()
        //     .filter(entity::accounts::Column::Id.is_in(account_ids))
        //     .all(&app_ctx.db)
        //     .await?
        //     .into_iter()
        //     .map(super::account::Account::from)
        //     .collect::<Vec<_>>();

        // Ok(accounts)
    }
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
