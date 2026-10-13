use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Queue {
    Table,
    Id,
    CampaignId,
    SubscriberId,
    Sent,
    QueryStr,
    HttpHeaders,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(
            Table::create()
                .table(Queue::Table)
                .if_not_exists()
                .col(ColumnDef::new(Queue::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(Queue::CampaignId).integer().null())
                .col(ColumnDef::new(Queue::SubscriberId).integer().null())
                .col(ColumnDef::new(Queue::Sent).small_integer().null())
                .col(ColumnDef::new(Queue::QueryStr).text().null())
                .col(ColumnDef::new(Queue::HttpHeaders).text().null())
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .name("s_id")
                .table(Queue::Table)
                .col(Queue::SubscriberId)
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .name("st_id")
                .table(Queue::Table)
                .col(Queue::Sent)
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .name("s_campaign_id_q")
                .table(Queue::Table)
                .col(Queue::CampaignId)
                .to_owned(),
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Queue::Table).if_exists().to_owned()).await
    }
}
