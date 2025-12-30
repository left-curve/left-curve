#[cfg(feature = "async-graphql")]
use {
    async_graphql::{ComplexObject, Context, Result, SimpleObject},
    grug_types::Timestamp,
};
use {
    dango_types::{account_factory::UserIndexDb, auth},
    sea_orm::entity::prelude::*,
    serde::{Deserialize, Serialize},
};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Hash, Serialize, Deserialize)]
#[sea_orm(table_name = "users_public_keys")]
#[cfg_attr(feature = "async-graphql", derive(SimpleObject))]
#[cfg_attr(feature = "async-graphql", graphql(complex))]
#[cfg_attr(feature = "async-graphql", graphql(name = "PublicKey"))]
#[cfg_attr(feature = "async-graphql", serde(rename_all = "camelCase"))]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[cfg_attr(
        all(feature = "async-graphql", not(feature = "testing")),
        graphql(skip)
    )]
    pub id: Uuid,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    pub user_index: UserIndexDb,
    pub key_hash: String,
    pub public_key: String,
    pub key_type: auth::KeyType,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "indexer_sql::serde_iso8601")]
    pub created_at: DateTime,
    pub created_block_height: i64,
}

#[cfg(feature = "async-graphql")]
#[ComplexObject]
impl Model {
    /// Returns the account creation timestamp in ISO 8601 format with time zone.
    async fn created_at(&self) -> String {
        Timestamp::from(self.created_at).to_rfc3339_string()
    }

    async fn user_index(&self) -> Result<u32> {
        Ok(self.user_index.try_into()?)
    }

    pub async fn users(&self, ctx: &Context<'_>) -> Result<Vec<crate::entity::users::Model>> {
        let db = ctx.data::<DatabaseConnection>()?;

        Ok(self
            .find_related(crate::entity::users::Entity)
            .all(db)
            .await?)

        // TODO: keeping the old code for reference

        // let user = entity::users::Entity::find()
        //     .filter(entity::users::Column::UserIndex.eq(&self.user_index))
        //     .one(&app_ctx.db)
        //     .await?
        //     .ok_or_else(|| async_graphql::Error::new("User not found"))?;

        // Ok(super::user::User::from(user))
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserIndex",
        to = "super::users::Column::UserIndex"
    )]
    User,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
