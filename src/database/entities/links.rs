use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "links")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub campaign_id: Option<i32>,
    pub ares_emails_id: Option<i32>,
    pub link: Option<String>,
    pub clicks: Option<String>,
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
        belongs_to = "super::ares_emails::Entity",
        from = "Column::AresEmailsId",
        to = "super::ares_emails::Column::Id"
    )]
    AresEmails,
}

impl Related<super::campaigns::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Campaigns.def()
    }
}

impl Related<super::ares_emails::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AresEmails.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
