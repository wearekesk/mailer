use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "webhooks_log")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub rule: Option<i32>,
    pub endpoint: Option<String>,
    pub payload: Option<String>,
    pub status_code: Option<String>,
    pub status_message: Option<String>,
    pub timestamp: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::rules::Entity",
        from = "Column::Rule",
        to = "super::rules::Column::Id"
    )]
    Rules,
}

impl Related<super::rules::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Rules.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
