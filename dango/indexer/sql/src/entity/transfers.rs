#[cfg(feature = "async-graphql")]
use async_graphql::{
    ComplexObject, Context, ErrorExtensions, Result as GraphQLResult, SimpleObject,
};
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
    ) -> GraphQLResult<crate::entity::accounts::Model> {
        let db = ctx.data::<DatabaseConnection>()?;

        self.find_account_by_address(db, &self.from_address)
            .await?
            .ok_or_else(|| {
                async_graphql::Error::new(format!(
                    "account with address {} not found. This is not expected.",
                    self.from_address
                ))
                .extend_with(|_err, e| e.set("code", "NOT_FOUND"))
            })
    }

    pub async fn to_account(
        &self,
        ctx: &Context<'_>,
    ) -> GraphQLResult<crate::entity::accounts::Model> {
        let db = ctx.data::<DatabaseConnection>()?;

        self.find_account_by_address(db, &self.to_address)
            .await?
            .ok_or_else(|| {
                async_graphql::Error::new(format!(
                    "account with address {} not found. This is not expected.",
                    self.to_address
                ))
                .extend_with(|_err, e| e.set("code", "NOT_FOUND"))
            })
    }
}

impl Model {
    async fn find_account_by_address(
        &self,
        db: &DatabaseConnection,
        address: &str,
    ) -> Result<Option<crate::entity::accounts::Model>, sea_orm::DbErr> {
        crate::entity::accounts::Entity::find()
            .filter(crate::entity::accounts::Column::Address.eq(address))
            .one(db)
            .await
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
