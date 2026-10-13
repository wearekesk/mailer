use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum WebhooksLog {
    Table,
    Id,
    Rule,
    Timestamp,
    Endpoint,
    StatusCode,
    Payload,
    StatusMessage,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WebhooksLog::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WebhooksLog::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WebhooksLog::Rule).integer().null())
                    .col(ColumnDef::new(WebhooksLog::Timestamp).big_integer().null())
                    .col(ColumnDef::new(WebhooksLog::Endpoint).string_len(255).null())
                    .col(ColumnDef::new(WebhooksLog::StatusCode).string_len(10).null())
                    .col(ColumnDef::new(WebhooksLog::Payload).text().null())
                    .col(ColumnDef::new(WebhooksLog::StatusMessage).text().null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("s_rule")
                    .table(WebhooksLog::Table)
                    .col(WebhooksLog::Rule)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("s_status_code")
                    .table(WebhooksLog::Table)
                    .col(WebhooksLog::StatusCode)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(WebhooksLog::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
