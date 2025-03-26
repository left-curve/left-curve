use {crate::idens::Block, sea_orm_migration::prelude::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Block::Table)
                    .add_column(
                        ColumnDef::new(Block::ProposerAddress)
                            .string()
                            .not_null()
                            .default("".to_string()),
                    )
                    .to_owned(),
            )
            .await?;

        // TODO: add existing proposer_address from blocks

        manager
            .alter_table(
                Table::alter()
                    .table(Block::Table)
                    .modify_column(ColumnDef::new(Block::ProposerAddress).string().not_null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Block::Table)
                    .drop_column(Block::ProposerAddress)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
