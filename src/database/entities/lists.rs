use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "lists")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub app: Option<i32>,
    pub user_id: Option<i32>,
    pub name: Option<String>,
    pub opt_in: Option<i16>,
    pub confirm_url: Option<String>,
    pub subscribed_url: Option<String>,
    pub unsubscribed_url: Option<String>,
    pub no_consent_url: Option<String>,
    pub already_subscribed_url: Option<String>,
    pub reconsent_success_url: Option<String>,
    pub thankyou: Option<i16>,
    pub thankyou_subject: Option<String>,
    pub thankyou_message: Option<String>,
    pub goodbye: Option<i16>,
    pub goodbye_subject: Option<String>,
    pub goodbye_message: Option<String>,
    pub unsubscribe_all_list: Option<i16>,
    pub unsubscribe_confirm: Option<i16>,
    pub confirmation_subject: Option<String>,
    pub confirmation_email: Option<String>,
    pub custom_fields: Option<String>,
    pub prev_count: Option<i32>,
    pub currently_processing: Option<i32>,
    pub total_records: Option<i32>,
    pub gdpr_enabled: Option<i16>,
    pub marketing_permission: Option<String>,
    pub what_to_expect: Option<String>,
    pub gdpr: Option<i16>,
    pub hide: Option<i16>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::database::entities::apps::Entity",
        from = "Column::App",
        to = "crate::database::entities::apps::Column::Id"
    )]
    Apps,
    #[sea_orm(has_many = "crate::database::entities::subscribers::Entity")]
    Subscribers,
}

impl Related<crate::database::entities::apps::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Apps.def()
    }
}

impl Related<crate::database::entities::subscribers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Subscribers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
