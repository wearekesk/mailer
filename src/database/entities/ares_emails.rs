use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "ares_emails")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub ares_id: Option<i32>,
    pub from_name: Option<String>,
    pub from_email: Option<String>,
    pub reply_to: Option<String>,
    pub title: Option<String>,
    pub plain_text: Option<String>,
    pub html_text: Option<String>,
    pub query_string: Option<String>,
    pub time_condition: Option<String>,
    pub timezone: Option<String>,
    pub created: Option<i64>,
    pub recipients: Option<i32>,
    pub opens: Option<String>,
    pub wysiwyg: Option<i16>,
    pub opens_tracking: Option<i16>,
    pub links_tracking: Option<i16>,
    pub enabled: Option<i16>,
    pub segs: Option<String>,
    pub segs_excl: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::ares::Entity",
        from = "Column::AresId",
        to = "super::ares::Column::Id"
    )]
    Ares,
}

impl Related<super::ares::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Ares.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
