use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "queue")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub query_str: Option<String>,
    pub http_headers: Option<String>,
    pub campaign_id: Option<i32>,
    pub subscriber_id: Option<i32>,
    pub sent: Option<i16>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::campaigns::Entity",
        from = "Column::CampaignId",
        to = "super::campaigns::Column::Id"
    )]
    Campaigns,
    #[sea_orm(
        belongs_to = "super::subscribers::Entity",
        from = "Column::SubscriberId",
        to = "super::subscribers::Column::Id"
    )]
    Subscribers,
}

impl Related<super::campaigns::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Campaigns.def()
    }
}

impl Related<super::subscribers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Subscribers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
