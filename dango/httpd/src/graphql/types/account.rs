use {
    super::user::User,
    async_graphql::*,
    chrono::{DateTime, TimeZone, Utc},
    dango_indexer_sql::entity,
    dango_types::account_factory,
    indexer_httpd::context::Context,
    sea_orm::{ModelTrait, sqlx::types::uuid},
};

#[derive(Clone, Debug, SimpleObject, Eq, PartialEq, Hash)]
#[graphql(complex)]
pub struct Account {
    #[graphql(skip)]
    pub id: uuid::Uuid,
    #[graphql(skip)]
    pub model: entity::accounts::Model,
    pub account_index: u32,
    pub address: String,
    pub account_type: account_factory::AccountType,
    pub created_at: DateTime<Utc>,
    pub created_block_height: u64,
}

impl From<entity::accounts::Model> for Account {
    fn from(item: entity::accounts::Model) -> Self {
        Self {
            id: item.id,
            model: item.clone(),
            created_at: Utc.from_utc_datetime(&item.created_at),
            address: item.address,
            account_type: item.account_type,
            created_block_height: item.created_block_height as u64,
            account_index: item.account_index as u32,
        }
    }
}

#[ComplexObject]
impl Account {
    pub async fn users(&self, ctx: &async_graphql::Context<'_>) -> Result<Vec<User>> {
        let app_ctx = ctx.data::<Context>()?;

        Ok(self
            .model
            .find_related(entity::users::Entity)
            .all(&app_ctx.db)
            .await?
            .into_iter()
            .map(User::from)
            .collect::<Vec<_>>())

        // TODO: keeping the old code for reference in case the join query doesn't work

        // let user_ids = entity::accounts_users::Entity::find()
        //     .filter(entity::accounts_users::Column::AccountId.eq(self.id))
        //     .all(&app_ctx.db)
        //     .await?
        //     .into_iter()
        //     .map(|au| au.user_id)
        //     .collect::<Vec<_>>();

        // let users = entity::users::Entity::find()
        //     .filter(entity::users::Column::Id.is_in(user_ids))
        //     .all(&app_ctx.db)
        //     .await?
        //     .into_iter()
        //     .map(User::from)
        //     .collect::<Vec<_>>();

        // Ok(users)
    }
}
