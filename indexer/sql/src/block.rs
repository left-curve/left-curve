use {
    crate::entity::{self, blocks::Entity},
    sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder},
};

impl Entity {
    pub async fn find_last_block_height<C>(db: &C) -> Result<Option<i64>, sea_orm::DbErr>
    where
        C: ConnectionTrait,
    {
        let block = entity::blocks::Entity::find()
            .order_by_desc(crate::entity::blocks::Column::BlockHeight)
            .one(db)
            .await?;

        Ok(block.map(|b| b.block_height))
    }

    /// Delete a block and its related content from the database
    pub async fn delete_block_and_data<C>(db: &C, block_height: u64) -> Result<(), sea_orm::DbErr>
    where
        C: ConnectionTrait,
    {
        entity::blocks::Entity::delete_many()
            .filter(entity::blocks::Column::BlockHeight.eq(block_height))
            .exec(db)
            .await?;

        entity::transactions::Entity::delete_many()
            .filter(entity::transactions::Column::BlockHeight.eq(block_height))
            .exec(db)
            .await?;

        entity::messages::Entity::delete_many()
            .filter(entity::messages::Column::BlockHeight.eq(block_height))
            .exec(db)
            .await?;

        entity::events::Entity::delete_many()
            .filter(entity::events::Column::BlockHeight.eq(block_height))
            .exec(db)
            .await?;

        Ok(())
    }
}
