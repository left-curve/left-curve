use {
    async_graphql::*,
    chrono::{DateTime, TimeZone, Utc},
    dango_indexer_sql::entity,
    sea_orm::ModelTrait,
};

#[derive(Enum, Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum KeyType {
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

#[derive(Clone, Debug, SimpleObject, Eq, PartialEq, Hash)]
#[graphql(complex)]
pub struct PublicKey {
    #[graphql(skip)]
    pub model: entity::public_keys::Model,
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
            model: item.clone(),
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
    pub async fn users(&self, ctx: &async_graphql::Context<'_>) -> Result<Vec<super::user::User>> {
        let app_ctx = ctx.data::<indexer_httpd::context::Context>()?;

        Ok(self
            .model
            .find_related(entity::users::Entity)
            .all(&app_ctx.db)
            .await?
            .into_iter()
            .map(super::user::User::from)
            .collect())

        // TODO: keeping the old code for reference

        // let user = entity::users::Entity::find()
        //     .filter(entity::users::Column::Username.eq(&self.username))
        //     .one(&app_ctx.db)
        //     .await?
        //     .ok_or_else(|| async_graphql::Error::new("User not found"))?;

        // Ok(super::user::User::from(user))
    }
}
