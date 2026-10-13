use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum SuppressionList {
    Table,
    Id,
    App,
    BlockAttempts,
    Timestamp,
    Email,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SuppressionList::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SuppressionList::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SuppressionList::App).integer().null())
                    .col(ColumnDef::new(SuppressionList::BlockAttempts).integer().null())
                    .col(ColumnDef::new(SuppressionList::Timestamp).big_integer().null())
                    .col(ColumnDef::new(SuppressionList::Email).string_len(100).null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(SuppressionList::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
