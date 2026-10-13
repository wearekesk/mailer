use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Subscribers {
    Table,
    Id,
    UserId,
    List,
    BounceSoft,
    LastCampaign,
    LastAres,
    Method,
    AddedVia,
    Unsubscribed,
    Bounced,
    Complaint,
    Confirmed,
    Gdpr,
    Timestamp,
    JoinDate,
    Name,
    Email,
    MessageId,
    Ip,
    Country,
    Referrer,
    CustomFields,
    Notes,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(
            Table::create()
                .table(Subscribers::Table)
                .if_not_exists()
                .col(ColumnDef::new(Subscribers::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(Subscribers::UserId).integer().null())
                .col(ColumnDef::new(Subscribers::List).integer().null())
                .col(ColumnDef::new(Subscribers::BounceSoft).integer().null())
                .col(ColumnDef::new(Subscribers::LastCampaign).integer().null())
                .col(ColumnDef::new(Subscribers::LastAres).integer().null())
                .col(ColumnDef::new(Subscribers::Method).small_integer().null())
                .col(ColumnDef::new(Subscribers::AddedVia).small_integer().null())
                .col(ColumnDef::new(Subscribers::Unsubscribed).small_integer().null())
                .col(ColumnDef::new(Subscribers::Bounced).small_integer().null())
                .col(ColumnDef::new(Subscribers::Complaint).small_integer().null())
                .col(ColumnDef::new(Subscribers::Confirmed).small_integer().null())
                .col(ColumnDef::new(Subscribers::Gdpr).small_integer().null())
                .col(ColumnDef::new(Subscribers::Timestamp).big_integer().null())
                .col(ColumnDef::new(Subscribers::JoinDate).big_integer().null())
                .col(ColumnDef::new(Subscribers::Name).string_len(100).null())
                .col(ColumnDef::new(Subscribers::Email).string_len(100).null())
                .col(ColumnDef::new(Subscribers::MessageId).string_len(100).null())
                .col(ColumnDef::new(Subscribers::Ip).string_len(100).null())
                .col(ColumnDef::new(Subscribers::Country).string_len(2).null())
                .col(ColumnDef::new(Subscribers::Referrer).string_len(500).null())
                .col(ColumnDef::new(Subscribers::CustomFields).text().null())
                .col(ColumnDef::new(Subscribers::Notes).text().null())
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .name("s_list")
                .table(Subscribers::Table)
                .col(Subscribers::List)
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .name("s_email")
                .table(Subscribers::Table)
                .col(Subscribers::Email)
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .name("s_unsubscribed")
                .table(Subscribers::Table)
                .col(Subscribers::Unsubscribed)
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .name("s_bounced")
                .table(Subscribers::Table)
                .col(Subscribers::Bounced)
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .name("s_bounce_soft")
                .table(Subscribers::Table)
                .col(Subscribers::BounceSoft)
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .name("s_complaint")
                .table(Subscribers::Table)
                .col(Subscribers::Complaint)
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .name("s_confirmed")
                .table(Subscribers::Table)
                .col(Subscribers::Confirmed)
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .name("s_timestamp")
                .table(Subscribers::Table)
                .col(Subscribers::Timestamp)
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .name("s_last_campaign")
                .table(Subscribers::Table)
                .col(Subscribers::LastCampaign)
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .name("s_messageid")
                .table(Subscribers::Table)
                .col(Subscribers::MessageId)
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .name("s_country")
                .table(Subscribers::Table)
                .col(Subscribers::Country)
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .name("s_referrer")
                .table(Subscribers::Table)
                .col(Subscribers::Referrer)
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .name("s_method")
                .table(Subscribers::Table)
                .col(Subscribers::Method)
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .name("s_added_via")
                .table(Subscribers::Table)
                .col(Subscribers::AddedVia)
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .name("s_gdpr")
                .table(Subscribers::Table)
                .col(Subscribers::Gdpr)
                .to_owned(),
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Subscribers::Table).if_exists().to_owned()).await
    }
}
