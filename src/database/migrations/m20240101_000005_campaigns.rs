use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Campaigns {
    Table,
    Id,
    UserId,
    App,
    ToSend,
    Recipients,
    QuotaDeducted,
    Wysiwyg,
    BounceSetup,
    ComplaintSetup,
    OpensTracking,
    LinksTracking,
    CampaignStopped,
    IgnoreChecks,
    FromName,
    FromEmail,
    ReplyTo,
    TimeoutCheck,
    Timezone,
    SendDate,
    Sent,
    Title,
    Label,
    Subject,
    QueryString,
    WebVersionLang,
    PlainText,
    HtmlText,
    ToSendLists,
    Opens,
    Errors,
    Lists,
    ListsExcl,
    Segs,
    SegsExcl,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(
            Table::create()
                .table(Campaigns::Table)
                .if_not_exists()
                .col(ColumnDef::new(Campaigns::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(Campaigns::UserId).integer().null())
                .col(ColumnDef::new(Campaigns::App).integer().null())
                .col(ColumnDef::new(Campaigns::ToSend).integer().null())
                .col(ColumnDef::new(Campaigns::Recipients).integer().null())
                .col(ColumnDef::new(Campaigns::QuotaDeducted).integer().null())
                .col(ColumnDef::new(Campaigns::Wysiwyg).small_integer().null())
                .col(ColumnDef::new(Campaigns::BounceSetup).small_integer().null())
                .col(ColumnDef::new(Campaigns::ComplaintSetup).small_integer().null())
                .col(ColumnDef::new(Campaigns::OpensTracking).small_integer().null())
                .col(ColumnDef::new(Campaigns::LinksTracking).small_integer().null())
                .col(ColumnDef::new(Campaigns::CampaignStopped).small_integer().null())
                .col(ColumnDef::new(Campaigns::IgnoreChecks).small_integer().null())
                .col(ColumnDef::new(Campaigns::FromName).string_len(100).null())
                .col(ColumnDef::new(Campaigns::FromEmail).string_len(100).null())
                .col(ColumnDef::new(Campaigns::ReplyTo).string_len(100).null())
                .col(ColumnDef::new(Campaigns::TimeoutCheck).string_len(100).null())
                .col(ColumnDef::new(Campaigns::Timezone).string_len(100).null())
                .col(ColumnDef::new(Campaigns::SendDate).string_len(100).null())
                .col(ColumnDef::new(Campaigns::Sent).string_len(100).null())
                .col(ColumnDef::new(Campaigns::Title).string_len(500).null())
                .col(ColumnDef::new(Campaigns::Label).string_len(500).null())
                .col(ColumnDef::new(Campaigns::Subject).string_len(500).null())
                .col(ColumnDef::new(Campaigns::QueryString).string_len(500).null())
                .col(ColumnDef::new(Campaigns::WebVersionLang).string_len(50).null())
                .col(ColumnDef::new(Campaigns::PlainText).text().null())
                .col(ColumnDef::new(Campaigns::HtmlText).text().null())
                .col(ColumnDef::new(Campaigns::ToSendLists).text().null())
                .col(ColumnDef::new(Campaigns::Opens).text().null())
                .col(ColumnDef::new(Campaigns::Errors).text().null())
                .col(ColumnDef::new(Campaigns::Lists).text().null())
                .col(ColumnDef::new(Campaigns::ListsExcl).text().null())
                .col(ColumnDef::new(Campaigns::Segs).text().null())
                .col(ColumnDef::new(Campaigns::SegsExcl).text().null())
                .to_owned(),
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Campaigns::Table).if_exists().to_owned()).await
    }
}
