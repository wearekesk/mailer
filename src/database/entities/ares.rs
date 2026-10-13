use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "ares")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub name: Option<String>,
    #[sea_orm(column_name = "type")]
    pub r#type: Option<i32>,
    pub list: Option<i32>,
    pub custom_field: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::lists::Entity",
        from = "Column::List",
        to = "super::lists::Column::Id"
    )]
    Lists,
    #[sea_orm(has_many = "super::ares_emails::Entity")]
    AresEmails,
}

impl Related<super::lists::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Lists.def()
    }
}

impl Related<super::ares_emails::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AresEmails.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
