use sea_orm::{QueryOrder, entity::prelude::*};

#[derive(
    Clone,
    Debug,
    PartialEq,
    DeriveEntityModel,
    Eq,
    Default,
    serde :: Serialize,
    serde :: Deserialize,
)]
#[sea_orm(table_name = "blocks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub created_at: DateTime,
    #[sea_orm(unique)]
    pub block_height: i64,
    pub hash: String,
    pub app_hash: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub async fn latest_block_height<C>(db: &C) -> Result<Option<i64>, DbErr>
where
    C: ConnectionTrait,
{
    Ok(Entity::find()
        .order_by_desc(Column::BlockHeight)
        .one(db)
        .await?
        .map(|block| block.block_height))
}
