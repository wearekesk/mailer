use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Links {
    Table,
    Id,
    CampaignId,
    AresEmailsId,
    Link,
    Clicks,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(
            Table::create()
                .table(Links::Table)
                .if_not_exists()
                .col(ColumnDef::new(Links::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(Links::CampaignId).integer().null())
                .col(ColumnDef::new(Links::AresEmailsId).integer().null())
                .col(ColumnDef::new(Links::Link).string_len(1500).null())
                .col(ColumnDef::new(Links::Clicks).text().null())
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .name("s_campaign_id")
                .table(Links::Table)
                .col(Links::CampaignId)
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .name("s_ares_emails_id")
                .table(Links::Table)
                .col(Links::AresEmailsId)
                .to_owned(),
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Links::Table).if_exists().to_owned()).await
    }
}
