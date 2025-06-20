#[cfg(feature = "async-graphql")]
use async_graphql::{ComplexObject, Context, Result as GraphQLResult, SimpleObject};

use {
    sea_orm::{Order, QueryOrder, entity::prelude::*},
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
    pub async fn accounts(
        &self,
        ctx: &Context<'_>,
    ) -> GraphQLResult<Vec<crate::entity::accounts::Model>> {
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

    pub async fn from_account(
        &self,
        ctx: &Context<'_>,
    ) -> GraphQLResult<Option<crate::entity::accounts::Model>> {
        let db = ctx.data::<DatabaseConnection>()?;

        Ok(crate::entity::accounts::Model::find_account_by_address(db, &self.from_address).await?)
    }

    pub async fn to_account(
        &self,
        ctx: &Context<'_>,
    ) -> GraphQLResult<Option<crate::entity::accounts::Model>> {
        let db = ctx.data::<DatabaseConnection>()?;

        Ok(crate::entity::accounts::Model::find_account_by_address(db, &self.to_address).await?)
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl indexer_sql::entity::OrderByBlocks<Entity> for Select<Entity> {
    fn order_by_blocks(self, order: Order) -> Self {
        self.order_by(Column::BlockHeight, order.clone())
            .order_by(Column::Idx, order)
    }
}
