use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum SkippedEmails {
    Table,
    Id,
    App,
    List,
    Reason,
    Email,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SkippedEmails::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SkippedEmails::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SkippedEmails::App).integer().null())
                    .col(ColumnDef::new(SkippedEmails::List).integer().null())
                    .col(ColumnDef::new(SkippedEmails::Reason).small_integer().null())
                    .col(ColumnDef::new(SkippedEmails::Email).string_len(100).null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("s_list_skip")
                    .table(SkippedEmails::Table)
                    .col(SkippedEmails::List)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(SkippedEmails::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
