use {
    async_graphql::*,
    chrono::{DateTime, TimeZone, Utc},
    dango_indexer_sql::entity,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter},
    serde::Deserialize,
};

#[derive(Clone, Debug, SimpleObject, Deserialize, Default, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
#[graphql(complex)]
#[serde(default)]
pub struct Transfer {
    pub block_height: u64,
    pub idx: i32,
    pub created_at: DateTime<Utc>,
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    pub denom: String,
}

impl From<entity::transfers::Model> for Transfer {
    fn from(item: entity::transfers::Model) -> Self {
        Self {
            block_height: item.block_height as u64,
            idx: item.idx,
            created_at: Utc.from_utc_datetime(&item.created_at),
            from_address: item.from_address,
            to_address: item.to_address,
            amount: item.amount,
            denom: item.denom,
        }
    }
}

#[ComplexObject]
impl Transfer {
    pub async fn accounts(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> Result<Vec<super::account::Account>> {
        let app_ctx = ctx.data::<indexer_httpd::context::Context>()?;

        let accounts = entity::accounts::Entity::find()
            .filter(
                entity::accounts::Column::Address
                    .eq(&self.from_address)
                    .or(entity::accounts::Column::Address.eq(&self.to_address)),
            )
            .all(&app_ctx.db)
            .await?
            .into_iter()
            .map(super::account::Account::from)
            .collect::<Vec<_>>();

        Ok(accounts)
    }

    pub async fn from_accounts(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> Result<Vec<super::account::Account>> {
        let app_ctx = ctx.data::<indexer_httpd::context::Context>()?;

        let accounts = entity::accounts::Entity::find()
            .filter(entity::accounts::Column::Address.eq(&self.from_address))
            .all(&app_ctx.db)
            .await?
            .into_iter()
            .map(super::account::Account::from)
            .collect::<Vec<_>>();

        Ok(accounts)
    }

    pub async fn to_accounts(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> Result<Vec<super::account::Account>> {
        let app_ctx = ctx.data::<indexer_httpd::context::Context>()?;

        let accounts = entity::accounts::Entity::find()
            .filter(entity::accounts::Column::Address.eq(&self.to_address))
            .all(&app_ctx.db)
            .await?
            .into_iter()
            .map(super::account::Account::from)
            .collect::<Vec<_>>();

        Ok(accounts)
    }
}
