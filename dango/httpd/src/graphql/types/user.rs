use {
    super::public_key::PublicKey,
    async_graphql::*,
    chrono::{DateTime, TimeZone, Utc},
    dango_indexer_sql::entity,
    indexer_httpd::context::Context,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter},
    serde::Deserialize,
};

#[derive(Clone, Debug, SimpleObject, Deserialize, Default, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
#[graphql(complex)]
#[serde(default)]
pub struct User {
    pub username: String,
    pub created_at: DateTime<Utc>,
    pub created_block_height: u64,
}

impl From<entity::users::Model> for User {
    fn from(item: entity::users::Model) -> Self {
        Self {
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

        let public_keys = entity::public_keys::Entity::find()
            .filter(entity::public_keys::Column::Username.eq(&self.username))
            .all(&app_ctx.db)
            .await?
            .into_iter()
            .map(PublicKey::from)
            .collect::<Vec<_>>();

        Ok(public_keys)
    }
}
