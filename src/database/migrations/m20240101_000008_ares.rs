use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Ares {
    Table,
    Id,
    List,
    Name,
    CustomField,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Ares::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Ares::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("type")).integer().null())
                    .col(ColumnDef::new(Ares::List).integer().null())
                    .col(ColumnDef::new(Ares::Name).string_len(100).null())
                    .col(ColumnDef::new(Ares::CustomField).string_len(100).null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("s_list_ares")
                    .table(Ares::Table)
                    .col(Ares::List)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Ares::Table).if_exists().to_owned())
            .await
    }
}
