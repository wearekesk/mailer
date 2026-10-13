use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Seg {
    Table,
    Id,
    App,
    List,
    Name,
    LastUpdated,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Seg::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Seg::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Seg::App).integer().null())
                    .col(ColumnDef::new(Seg::List).integer().null())
                    .col(ColumnDef::new(Seg::Name).string_len(100).null())
                    .col(ColumnDef::new(Seg::LastUpdated).string_len(100).null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("s_list_seg")
                    .table(Seg::Table)
                    .col(Seg::List)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Seg::Table).if_exists().to_owned())
            .await
    }
}
