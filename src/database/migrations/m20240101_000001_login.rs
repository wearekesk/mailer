use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Login {
    Table,
    Id,
    Name,
    Company,
    Username,
    Password,
    S3Key,
    S3Secret,
    ApiKey,
    ApiKeyPrev,
    License,
    Timezone,
    DbUpdated,
    Paypal,
    Language,
    SesEndpoint,
    AuthKey,
    AuthSalt,
    ResetPasswordKey,
    TiedTo,
    App,
    SendRate,
    BrandsRows,
    Cron,
    CronAres,
    CronCsv,
    CronSeg,
    AuthEnabled,
    DarkMode,
    StrictDelete,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Login::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Login::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Login::Name).string_len(100).null())
                    .col(ColumnDef::new(Login::Company).string_len(100).null())
                    .col(ColumnDef::new(Login::Username).string_len(100).null())
                    .col(ColumnDef::new(Login::Password).string_len(500).null())
                    .col(ColumnDef::new(Login::S3Key).string_len(100).null())
                    .col(ColumnDef::new(Login::S3Secret).string_len(100).null())
                    .col(ColumnDef::new(Login::ApiKey).string_len(500).null())
                    .col(ColumnDef::new(Login::ApiKeyPrev).string_len(100).null())
                    .col(ColumnDef::new(Login::License).string_len(100).null())
                    .col(ColumnDef::new(Login::Timezone).string_len(100).null())
                    .col(ColumnDef::new(Login::DbUpdated).string_len(100).null())
                    .col(ColumnDef::new(Login::Paypal).string_len(100).null())
                    .col(ColumnDef::new(Login::Language).string_len(100).null())
                    .col(ColumnDef::new(Login::SesEndpoint).string_len(100).null())
                    .col(ColumnDef::new(Login::AuthKey).string_len(100).null())
                    .col(ColumnDef::new(Login::AuthSalt).string_len(20).null())
                    .col(
                        ColumnDef::new(Login::ResetPasswordKey)
                            .string_len(20)
                            .null(),
                    )
                    .col(ColumnDef::new(Login::TiedTo).integer().null())
                    .col(ColumnDef::new(Login::App).integer().null())
                    .col(ColumnDef::new(Login::SendRate).integer().null())
                    .col(ColumnDef::new(Login::BrandsRows).integer().null())
                    .col(ColumnDef::new(Login::Cron).small_integer().null())
                    .col(ColumnDef::new(Login::CronAres).small_integer().null())
                    .col(ColumnDef::new(Login::CronCsv).small_integer().null())
                    .col(ColumnDef::new(Login::CronSeg).small_integer().null())
                    .col(
                        ColumnDef::new(Login::AuthEnabled)
                            .small_integer()
                            .null(),
                    )
                    .col(ColumnDef::new(Login::DarkMode).small_integer().null())
                    .col(
                        ColumnDef::new(Login::StrictDelete)
                            .small_integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Login::Table).if_exists().to_owned())
            .await
    }
}
