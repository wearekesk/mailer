use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum BlockedDomains {
    Table,
    Id,
    App,
    BlockAttempts,
    Timestamp,
    Domain,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BlockedDomains::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BlockedDomains::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(BlockedDomains::App).integer().null())
                    .col(ColumnDef::new(BlockedDomains::BlockAttempts).integer().null())
                    .col(ColumnDef::new(BlockedDomains::Timestamp).big_integer().null())
                    .col(ColumnDef::new(BlockedDomains::Domain).string_len(100).null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(BlockedDomains::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
