use {
    crate::idens::{Block, Event, Message, Transaction},
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
                    .col(string(Block::AppHash))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Transaction::Table)
                    .if_not_exists()
                    .col(pk_uuid(Transaction::Id))
                    .col(integer(Transaction::TransactionType))
                    .col(integer(Transaction::TransactionIdx))
                    .col(date_time(Transaction::CreatedAt))
                    // TODO: add foreign key to blocks
                    .col(
                        ColumnDef::new(Transaction::BlockHeight)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(string(Transaction::Sender))
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
                    // TODO: add foreign key to transactions
                    .col(uuid(Message::TransactionId))
                    .col(integer(Message::OrderIdx))
                    .col(date_time(Message::CreatedAt))
                    .col(json_binary(Message::Data))
                    .col(string(Message::MethodName))
                    // TODO: add foreign key to blocks
                    .col(
                        ColumnDef::new(Message::BlockHeight)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(string_null(Message::ContractAddr))
                    .col(string(Message::SenderAddr))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Event::Table)
                    .if_not_exists()
                    .col(pk_uuid(Event::Id))
                    .col(uuid_null(Event::ParentId))
                    // TODO: add foreign key to transactions
                    .col(uuid_null(Event::TransactionId))
                    .col(uuid_null(Event::MessageId))
                    .col(date_time(Event::CreatedAt))
                    .col(string(Event::Type))
                    .col(string_null(Event::Method))
                    .col(integer(Event::EventStatus))
                    .col(integer(Event::CommitmentStatus))
                    .col(integer(Event::TransactionType))
                    .col(integer(Event::TransactionIdx))
                    .col(integer_null(Event::MessageIdx))
                    .col(integer(Event::EventIdx))
                    .col(json_binary(Event::Data))
                    // TODO: add foreign key to blocks
                    .col(ColumnDef::new(Event::BlockHeight).big_unsigned().not_null())
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

        manager
            .create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .name("events-block_height")
                    .table(Event::Table)
                    .col(Event::BlockHeight)
                    .to_owned(),
            )
            .await
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
            .await
    }
}
