#[cfg(feature = "async-graphql")]
use async_graphql::{ComplexObject, Result, SimpleObject};
use {async_graphql::Context, dango_types::account_factory, sea_orm::entity::prelude::*};

#[derive(
    Clone, Debug, PartialEq, DeriveEntityModel, Eq, Hash, serde :: Serialize, serde :: Deserialize,
)]
#[sea_orm(table_name = "accounts")]
#[cfg_attr(feature = "async-graphql", derive(SimpleObject))]
#[cfg_attr(feature = "async-graphql", graphql(complex))]
#[cfg_attr(feature = "async-graphql", graphql(name = "Account"))]
#[cfg_attr(feature = "async-graphql", serde(rename_all = "camelCase"))]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[cfg_attr(
        all(feature = "async-graphql", not(feature = "testing")),
        graphql(skip)
    )]
    pub id: Uuid,
    pub account_index: i32,
    #[sea_orm(unique)]
    pub address: String,
    pub account_type: account_factory::AccountType,
    pub created_at: DateTime,
    pub created_block_height: i64,
}

#[cfg(feature = "async-graphql")]
#[ComplexObject]
impl Model {
    pub async fn users(&self, ctx: &Context<'_>) -> Result<Vec<crate::entity::users::Model>> {
        let db = ctx.data::<DatabaseConnection>()?;

        Ok(self
            .find_related(crate::entity::users::Entity)
            .all(db)
            .await?)

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

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::accounts_users::Entity",
        from = "Column::Id",
        to = "super::accounts_users::Column::AccountId"
    )]
    AccountUser,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        super::accounts_users::Relation::User.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::accounts_users::Relation::Account.def().rev())
    }
}

impl Related<super::accounts_users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AccountUser.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
