use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "subscribers_seg")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub seg_id: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub subscriber_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::seg::Entity",
        from = "Column::SegId",
        to = "super::seg::Column::Id"
    )]
    Seg,
    #[sea_orm(
        belongs_to = "super::subscribers::Entity",
        from = "Column::SubscriberId",
        to = "super::subscribers::Column::Id"
    )]
    Subscribers,
}

impl Related<super::seg::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Seg.def()
    }
}

impl Related<super::subscribers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Subscribers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
