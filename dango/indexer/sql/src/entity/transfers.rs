#[cfg(feature = "async-graphql")]
use async_graphql::{ComplexObject, Context, Result, SimpleObject};
use {
    sea_orm::entity::prelude::*,
    serde::{Deserialize, Serialize},
};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Default, Serialize, Deserialize)]
#[sea_orm(table_name = "transfers")]
#[cfg_attr(feature = "async-graphql", derive(SimpleObject))]
#[cfg_attr(feature = "async-graphql", graphql(complex))]
#[cfg_attr(feature = "async-graphql", graphql(name = "Transfer"))]
#[cfg_attr(feature = "async-graphql", serde(rename_all = "camelCase"))]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[cfg_attr(
        all(feature = "async-graphql", not(feature = "testing")),
        graphql(skip)
    )]
    pub id: Uuid,
    pub idx: i32,
    pub created_at: DateTime,
    pub block_height: i64,
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    pub denom: String,
}

#[cfg(feature = "async-graphql")]
#[ComplexObject]
impl Model {
    pub async fn accounts(&self, ctx: &Context<'_>) -> Result<Vec<crate::entity::accounts::Model>> {
        let db = ctx.data::<DatabaseConnection>()?;

        let accounts = crate::entity::accounts::Entity::find()
            .filter(
                crate::entity::accounts::Column::Address
                    .eq(&self.from_address)
                    .or(crate::entity::accounts::Column::Address.eq(&self.to_address)),
            )
            .all(db)
            .await?;

        Ok(accounts)
    }

    pub async fn from_accounts(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<crate::entity::accounts::Model>> {
        let db = ctx.data::<DatabaseConnection>()?;

        let accounts = crate::entity::accounts::Entity::find()
            .filter(crate::entity::accounts::Column::Address.eq(&self.from_address))
            .all(db)
            .await?;

        Ok(accounts)
    }

    pub async fn to_accounts(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<crate::entity::accounts::Model>> {
        let db = ctx.data::<DatabaseConnection>()?;

        let accounts = crate::entity::accounts::Entity::find()
            .filter(crate::entity::accounts::Column::Address.eq(&self.to_address))
            .all(db)
            .await?;

        Ok(accounts)
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
