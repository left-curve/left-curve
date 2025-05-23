use {
    super::{account::Account, public_key::PublicKey},
    async_graphql::*,
    chrono::{DateTime, TimeZone, Utc},
    dango_indexer_sql::entity,
    indexer_httpd::context::Context,
    sea_orm::ModelTrait,
};

#[derive(Clone, Debug, SimpleObject, Eq, PartialEq, Hash)]
#[graphql(complex)]
pub struct User {
    #[graphql(skip)]
    pub model: entity::users::Model,
    pub username: String,
    pub created_at: DateTime<Utc>,
    pub created_block_height: u64,
}

impl From<entity::users::Model> for User {
    fn from(item: entity::users::Model) -> Self {
        Self {
            model: item.clone(),
            username: item.username,
            created_at: Utc.from_utc_datetime(&item.created_at),
            created_block_height: item.created_block_height as u64,
        }
    }
}

#[ComplexObject]
impl User {
    pub async fn public_keys(&self, ctx: &async_graphql::Context<'_>) -> Result<Vec<PublicKey>> {
        let app_ctx = ctx.data::<Context>()?;

        Ok(self
            .model
            .find_related(entity::public_keys::Entity)
            .all(&app_ctx.db)
            .await?
            .into_iter()
            .map(PublicKey::from)
            .collect::<Vec<_>>())

        // NOTE: leaving this here for now, in case the relation doesn't work
        // let public_keys = entity::public_keys::Entity::find()
        //     .filter(entity::public_keys::Column::Username.eq(&self.username))
        //     .all(&app_ctx.db)
        //     .await?
        //     .into_iter()
        //     .map(PublicKey::from)
        //     .collect::<Vec<_>>();

        // Ok(public_keys)
    }

    pub async fn accounts(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> Result<Vec<super::account::Account>> {
        let app_ctx = ctx.data::<Context>()?;

        Ok(self
            .model
            .find_related(entity::accounts::Entity)
            .all(&app_ctx.db)
            .await?
            .into_iter()
            .map(Account::from)
            .collect::<Vec<_>>())

        // NOTE: leaving this here for now, in case the relation doesn't work
        // let account_ids = entity::accounts_users::Entity::find()
        //     .filter(entity::accounts_users::Column::UserId.eq(self.username.clone()))
        //     .all(&app_ctx.db)
        //     .await?
        //     .into_iter()
        //     .map(|item| item.account_id)
        //     .collect::<Vec<_>>();

        // NOTE: leaving this here for now, in case the relation doesn't work
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
