use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum SegCons {
    Table,
    Id,
    SegId,
    GroupId,
    Operator,
    Field,
    Comparison,
    Val,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SegCons::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SegCons::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SegCons::SegId).integer().null())
                    .col(ColumnDef::new(SegCons::GroupId).integer().null())
                    .col(ColumnDef::new(SegCons::Operator).char_len(3).null())
                    .col(ColumnDef::new(SegCons::Field).string_len(100).null())
                    .col(ColumnDef::new(SegCons::Comparison).string_len(11).null())
                    .col(ColumnDef::new(SegCons::Val).string_len(500).null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("s_seg_id")
                    .table(SegCons::Table)
                    .col(SegCons::SegId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(SegCons::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
