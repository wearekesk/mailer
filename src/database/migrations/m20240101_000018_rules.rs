use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Rules {
    Table,
    Id,
    Brand,
    UnsubscribeListId,
    List,
    App,
    AresId,
    Enabled,
    Action,
    NotificationEmail,
    Endpoint,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Rules::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Rules::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Rules::Brand).integer().null())
                    .col(ColumnDef::new(Rules::UnsubscribeListId).integer().null())
                    .col(ColumnDef::new(Rules::List).integer().null())
                    .col(ColumnDef::new(Rules::App).integer().null())
                    .col(ColumnDef::new(Rules::AresId).integer().null())
                    .col(ColumnDef::new(Rules::Enabled).small_integer().null())
                    .col(ColumnDef::new(Alias::new("trigger")).string_len(100).null())
                    .col(ColumnDef::new(Rules::Action).string_len(100).null())
                    .col(ColumnDef::new(Rules::NotificationEmail).string_len(100).null())
                    .col(ColumnDef::new(Rules::Endpoint).string_len(255).null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("s_brand")
                    .table(Rules::Table)
                    .col(Rules::Brand)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("s_trigger")
                    .table(Rules::Table)
                    .col(Alias::new("trigger"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("s_action")
                    .table(Rules::Table)
                    .col(Rules::Action)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("s_list_rules")
                    .table(Rules::Table)
                    .col(Rules::List)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("s_app_rules")
                    .table(Rules::Table)
                    .col(Rules::App)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(Rules::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
