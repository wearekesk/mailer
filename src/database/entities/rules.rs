use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "rules")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub brand: Option<i32>,
    #[sea_orm(column_name = "trigger")]
    pub trigger_col: Option<String>,
    pub action: Option<String>,
    pub endpoint: Option<String>,
    pub notification_email: Option<String>,
    pub unsubscribe_list_id: Option<i32>,
    pub list: Option<i32>,
    pub app: Option<i32>,
    pub ares_id: Option<i32>,
    pub enabled: Option<i16>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::apps::Entity",
        from = "Column::App",
        to = "super::apps::Column::Id"
    )]
    Apps,
}

impl Related<super::apps::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Apps.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
