use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "seg")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub name: Option<String>,
    pub app: Option<i32>,
    pub list: Option<i32>,
    pub last_updated: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::apps::Entity",
        from = "Column::App",
        to = "super::apps::Column::Id"
    )]
    Apps,
    #[sea_orm(
        belongs_to = "super::lists::Entity",
        from = "Column::List",
        to = "super::lists::Column::Id"
    )]
    Lists,
    #[sea_orm(has_many = "super::seg_cons::Entity")]
    SegCons,
}

impl Related<super::apps::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Apps.def()
    }
}

impl Related<super::lists::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Lists.def()
    }
}

impl Related<super::seg_cons::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SegCons.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
