use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "login")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub name: Option<String>,
    pub company: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub s3_key: Option<String>,
    pub s3_secret: Option<String>,
    pub api_key: Option<String>,
    pub api_key_prev: Option<String>,
    pub license: Option<String>,
    pub timezone: Option<String>,
    pub db_updated: Option<String>,
    pub tied_to: Option<i32>,
    pub app: Option<i32>,
    pub paypal: Option<String>,
    pub cron: Option<i16>,
    pub cron_ares: Option<i16>,
    pub cron_csv: Option<i16>,
    pub cron_seg: Option<i16>,
    pub send_rate: Option<i32>,
    pub language: Option<String>,
    pub ses_endpoint: Option<String>,
    pub auth_enabled: Option<i16>,
    pub auth_key: Option<String>,
    pub auth_salt: Option<String>,
    pub reset_password_key: Option<String>,
    pub brands_rows: Option<i32>,
    pub dark_mode: Option<i16>,
    pub strict_delete: Option<i16>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::apps::Entity")]
    Apps,
}

impl Related<super::apps::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Apps.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
