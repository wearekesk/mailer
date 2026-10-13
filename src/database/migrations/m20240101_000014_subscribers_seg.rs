use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum SubscribersSeg {
    Table,
    SegId,
    SubscriberId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SubscribersSeg::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(SubscribersSeg::SegId).integer().not_null())
                    .col(ColumnDef::new(SubscribersSeg::SubscriberId).integer().not_null())
                    .primary_key(
                        Index::create()
                            .col(SubscribersSeg::SegId)
                            .col(SubscribersSeg::SubscriberId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("s_sid")
                    .table(SubscribersSeg::Table)
                    .col(SubscribersSeg::SegId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("s_subscriber_id")
                    .table(SubscribersSeg::Table)
                    .col(SubscribersSeg::SubscriberId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(SubscribersSeg::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
