use {
    async_graphql::*,
    chrono::{DateTime, TimeZone, Utc},
    dango_indexer_sql::entity,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter},
    serde::Deserialize,
};

#[derive(Enum, Copy, Clone, Debug, Deserialize, Default, Eq, PartialEq, Hash)]
pub enum KeyType {
    #[default]
    Secp256r1,
    Secp256k1,
    Ethereum,
}

impl From<entity::public_keys::KeyType> for KeyType {
    fn from(key_type: entity::public_keys::KeyType) -> KeyType {
        match key_type {
            entity::public_keys::KeyType::Secp256r1 => KeyType::Secp256r1,
            entity::public_keys::KeyType::Secp256k1 => KeyType::Secp256k1,
            entity::public_keys::KeyType::Ethereum => KeyType::Ethereum,
        }
    }
}

#[derive(Clone, Debug, SimpleObject, Deserialize, Default, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
#[graphql(complex)]
#[serde(default)]
pub struct PublicKey {
    pub username: String,
    pub key_hash: String,
    pub public_key: String,
    pub key_type: KeyType,
    pub created_at: DateTime<Utc>,
    pub created_block_height: u64,
}

impl From<entity::public_keys::Model> for PublicKey {
    fn from(item: entity::public_keys::Model) -> Self {
        Self {
            created_at: Utc.from_utc_datetime(&item.created_at),
            username: item.username,
            key_hash: item.key_hash,
            public_key: item.public_key,
            key_type: item.key_type.into(),
            created_block_height: item.created_block_height as u64,
        }
    }
}

#[ComplexObject]
impl PublicKey {
    pub async fn user(&self, ctx: &async_graphql::Context<'_>) -> Result<super::user::User> {
        let app_ctx = ctx.data::<indexer_httpd::context::Context>()?;

        let user = entity::users::Entity::find()
            .filter(entity::users::Column::Username.eq(&self.username))
            .one(&app_ctx.db)
            .await?
            .ok_or_else(|| async_graphql::Error::new("User not found"))?;

        Ok(super::user::User::from(user))
    }
}
