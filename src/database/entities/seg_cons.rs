use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "seg_cons")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub seg_id: Option<i32>,
    pub group_id: Option<i32>,
    pub operator: Option<String>,
    pub field: Option<String>,
    pub comparison: Option<String>,
    pub val: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::database::entities::seg::Entity",
        from = "Column::SegId",
        to = "crate::database::entities::seg::Column::Id"
    )]
    Seg,
}

impl Related<crate::database::entities::seg::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Seg.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
