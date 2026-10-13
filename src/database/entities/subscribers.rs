use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "subscribers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub user_id: Option<i32>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub list: Option<i32>,
    pub custom_fields: Option<String>,
    pub unsubscribed: Option<i16>,
    pub bounced: Option<i16>,
    pub bounce_soft: Option<i32>,
    pub complaint: Option<i16>,
    pub confirmed: Option<i16>,
    pub last_campaign: Option<i32>,
    pub last_ares: Option<i32>,
    pub timestamp: Option<i64>,
    pub join_date: Option<i64>,
    pub message_id: Option<String>,
    pub ip: Option<String>,
    pub country: Option<String>,
    pub referrer: Option<String>,
    pub method: Option<i16>,
    pub added_via: Option<i16>,
    pub notes: Option<String>,
    pub gdpr: Option<i16>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::lists::Entity",
        from = "Column::List",
        to = "super::lists::Column::Id"
    )]
    Lists,
}

impl Related<super::lists::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Lists.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
