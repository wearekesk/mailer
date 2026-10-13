use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Template {
    Table,
    Id,
    UserId,
    App,
    TemplateName,
    FromName,
    FromEmail,
    ReplyTo,
    HtmlText,
    PlainText,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Template::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Template::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Template::UserId).integer().null())
                    .col(ColumnDef::new(Template::App).integer().null())
                    .col(ColumnDef::new(Template::TemplateName).string_len(500).null())
                    .col(ColumnDef::new(Template::FromName).string_len(100).null())
                    .col(ColumnDef::new(Template::FromEmail).string_len(100).null())
                    .col(ColumnDef::new(Template::ReplyTo).string_len(100).null())
                    .col(ColumnDef::new(Template::HtmlText).text().null())
                    .col(ColumnDef::new(Template::PlainText).text().null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(Template::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
