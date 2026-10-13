use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "campaigns")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub user_id: Option<i32>,
    pub app: Option<i32>,
    pub from_name: Option<String>,
    pub from_email: Option<String>,
    pub reply_to: Option<String>,
    pub title: Option<String>,
    pub label: Option<String>,
    pub subject: Option<String>,
    pub plain_text: Option<String>,
    pub html_text: Option<String>,
    pub query_string: Option<String>,
    pub sent: Option<String>,
    pub to_send: Option<i32>,
    pub to_send_lists: Option<String>,
    pub recipients: Option<i32>,
    pub opens: Option<String>,
    pub errors: Option<String>,
    pub wysiwyg: Option<i16>,
    pub timeout_check: Option<String>,
    pub lists: Option<String>,
    pub lists_excl: Option<String>,
    pub segs: Option<String>,
    pub segs_excl: Option<String>,
    pub timezone: Option<String>,
    pub send_date: Option<String>,
    pub bounce_setup: Option<i16>,
    pub complaint_setup: Option<i16>,
    pub opens_tracking: Option<i16>,
    pub links_tracking: Option<i16>,
    pub quota_deducted: Option<i32>,
    pub web_version_lang: Option<String>,
    pub campaign_stopped: Option<i16>,
    pub ignore_checks: Option<i16>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::apps::Entity",
        from = "Column::App",
        to = "super::apps::Column::Id"
    )]
    Apps,
    #[sea_orm(has_many = "super::links::Entity")]
    Links,
    #[sea_orm(has_many = "super::queue::Entity")]
    Queue,
}

impl Related<super::apps::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Apps.def()
    }
}

impl Related<super::links::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Links.def()
    }
}

impl Related<super::queue::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Queue.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
