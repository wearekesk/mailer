use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum AresEmails {
    Table,
    Id,
    AresId,
    Recipients,
    Created,
    Wysiwyg,
    OpensTracking,
    LinksTracking,
    Enabled,
    FromName,
    FromEmail,
    ReplyTo,
    TimeCondition,
    Timezone,
    Title,
    QueryString,
    PlainText,
    HtmlText,
    Opens,
    Segs,
    SegsExcl,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AresEmails::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AresEmails::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AresEmails::AresId).integer().null())
                    .col(ColumnDef::new(AresEmails::Recipients).integer().null())
                    .col(ColumnDef::new(AresEmails::Created).big_integer().null())
                    .col(ColumnDef::new(AresEmails::Wysiwyg).small_integer().null())
                    .col(ColumnDef::new(AresEmails::OpensTracking).small_integer().null())
                    .col(ColumnDef::new(AresEmails::LinksTracking).small_integer().null())
                    .col(ColumnDef::new(AresEmails::Enabled).small_integer().null())
                    .col(ColumnDef::new(AresEmails::FromName).string_len(100).null())
                    .col(ColumnDef::new(AresEmails::FromEmail).string_len(100).null())
                    .col(ColumnDef::new(AresEmails::ReplyTo).string_len(100).null())
                    .col(ColumnDef::new(AresEmails::TimeCondition).string_len(100).null())
                    .col(ColumnDef::new(AresEmails::Timezone).string_len(100).null())
                    .col(ColumnDef::new(AresEmails::Title).string_len(500).null())
                    .col(ColumnDef::new(AresEmails::QueryString).string_len(500).null())
                    .col(ColumnDef::new(AresEmails::PlainText).text().null())
                    .col(ColumnDef::new(AresEmails::HtmlText).text().null())
                    .col(ColumnDef::new(AresEmails::Opens).text().null())
                    .col(ColumnDef::new(AresEmails::Segs).text().null())
                    .col(ColumnDef::new(AresEmails::SegsExcl).text().null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("s_ares_id")
                    .table(AresEmails::Table)
                    .col(AresEmails::AresId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(AresEmails::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
