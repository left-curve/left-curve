#[cfg(feature = "async-graphql")]
use {
    async_graphql::{ComplexObject, Context, Result, SimpleObject},
    grug_types::Timestamp,
};
use {dango_types::account_factory, sea_orm::entity::prelude::*};

#[derive(
    Clone, Debug, PartialEq, DeriveEntityModel, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[sea_orm(table_name = "accounts")]
#[cfg_attr(feature = "async-graphql", derive(SimpleObject))]
#[cfg_attr(feature = "async-graphql", graphql(complex))]
#[cfg_attr(feature = "async-graphql", graphql(name = "Account"))]
#[cfg_attr(feature = "async-graphql", serde(rename_all = "camelCase"))]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[cfg_attr(
        all(feature = "async-graphql", not(feature = "testing")),
        graphql(skip)
    )]
    pub id: Uuid,
    pub account_index: i32,
    #[sea_orm(unique)]
    pub address: String,
    pub account_type: account_factory::AccountType,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "indexer_sql::serde_iso8601")]
    pub created_at: DateTime,
    pub created_block_height: i64,
    pub created_tx_hash: String,
}

#[cfg(feature = "async-graphql")]
#[ComplexObject]
impl Model {
    /// Returns the account creation timestamp in ISO 8601 format with time zone.
    async fn created_at(&self) -> String {
        Timestamp::from(self.created_at).to_rfc3339_string()
    }

    pub async fn users(&self, ctx: &Context<'_>) -> Result<Vec<crate::entity::users::Model>> {
        let db = ctx.data::<DatabaseConnection>()?;
        Ok(self
            .find_related(crate::entity::users::Entity)
            .all(db)
            .await?)

        // TODO: keeping the old code for reference in case the join query doesn't work

        // let user_ids = crate::entity::accounts_users::Entity::find()
        //     .filter(crate::entity::accounts_users::Column::AccountId.eq(self.id))
        //     .all(db)
        //     .await?
        //     .into_iter()
        //     .map(|au| au.user_id)
        //     .collect::<Vec<_>>();

        // let users = crate::entity::users::Entity::find()
        //     .filter(crate::entity::users::Column::Id.is_in(user_ids))
        //     .all(db)
        //     .await?;

        // Ok(users)
    }
}

impl Model {
    // to avoid unused function warning when async-graphql feature is not enabled
    #[allow(dead_code)]
    pub async fn find_account_by_address(
        db: &DatabaseConnection,
        address: &str,
    ) -> Result<Option<Self>, DbErr> {
        Entity::find()
            .filter(Column::Address.eq(address))
            .one(db)
            .await
    }
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
