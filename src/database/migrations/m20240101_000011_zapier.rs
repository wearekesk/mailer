use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Zapier {
    Table,
    Id,
    List,
    App,
    SubscribeEndpoint,
    Event,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Zapier::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Zapier::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Zapier::List).integer().null())
                    .col(ColumnDef::new(Zapier::App).integer().null())
                    .col(ColumnDef::new(Zapier::SubscribeEndpoint).string_len(100).null())
                    .col(ColumnDef::new(Zapier::Event).string_len(100).null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(Zapier::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
