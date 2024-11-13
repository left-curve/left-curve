use {
    crate::idens::{Block, Message, Transaction},
    sea_orm_migration::{prelude::*, schema::*},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Block::Table)
                    .if_not_exists()
                    .col(pk_uuid(Block::Id))
                    .col(date_time(Block::CreatedAt))
                    .col(
                        ColumnDef::new(Block::BlockHeight)
                            .big_unsigned()
                            .unique_key()
                            .not_null(),
                    )
                    .col(string(Block::Hash))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Transaction::Table)
                    .if_not_exists()
                    .col(pk_uuid(Transaction::Id))
                    .col(date_time(Transaction::CreatedAt))
                    .col(
                        ColumnDef::new(Transaction::BlockHeight)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(json_binary(Transaction::Data))
                    .col(json_binary(Transaction::Credential))
                    .col(string(Transaction::Hash))
                    .col(boolean(Transaction::HasSucceeded))
                    .col(string_null(Transaction::ErrorMessage))
                    .col(big_unsigned(Transaction::GasWanted))
                    .col(big_unsigned(Transaction::GasUsed))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Message::Table)
                    .if_not_exists()
                    .col(pk_uuid(Message::Id))
                    .col(date_time(Message::CreatedAt))
                    .col(
                        ColumnDef::new(Transaction::BlockHeight)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(json(Message::Data))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .name("blocks-block_height")
                    .unique()
                    .table(Block::Table)
                    .col(Block::BlockHeight)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .name("transactions-block_height")
                    .table(Transaction::Table)
                    .col(Transaction::BlockHeight)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .name("transactions-hash")
                    .table(Transaction::Table)
                    .col(Transaction::Hash)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .name("messages-block_height")
                    .table(Message::Table)
                    .col(Message::BlockHeight)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Block::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Transaction::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Message::Table).to_owned())
            .await?;
        Ok(())
    }
}
