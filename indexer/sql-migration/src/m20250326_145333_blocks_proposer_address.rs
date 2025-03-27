use {
    crate::idens::Block,
    sea_orm_migration::{prelude::*, sea_orm::DbBackend},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite does not support modifying columns
        if manager.get_database_backend() != DbBackend::Sqlite {
            manager
                .alter_table(
                    Table::alter()
                        .table(Block::Table)
                        .modify_column(ColumnDef::new(Block::ProposerAddress).string().not_null())
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite does not support modifying columns
        if manager.get_database_backend() != DbBackend::Sqlite {
            manager
                .alter_table(
                    Table::alter()
                        .table(Block::Table)
                        .modify_column(
                            ColumnDef::new(Block::ProposerAddress)
                                .string()
                                .not_null()
                                .default("".to_string()),
                        )
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }
}
