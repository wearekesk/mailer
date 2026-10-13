use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "apps")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub user_id: Option<i32>,
    pub app_name: Option<String>,
    pub from_name: Option<String>,
    pub from_email: Option<String>,
    pub reply_to: Option<String>,
    pub app_key: Option<String>,
    pub brand_logo_filename: Option<String>,
    pub currency: Option<String>,
    pub delivery_fee: Option<String>,
    pub cost_per_recipient: Option<String>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<String>,
    pub smtp_ssl: Option<String>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub allocated_quota: Option<i32>,
    pub current_quota: Option<i32>,
    pub day_of_reset: Option<i32>,
    pub month_of_next_reset: Option<String>,
    pub year_of_next_reset: Option<String>,
    pub no_expiry: Option<i16>,
    pub test_email: Option<String>,
    pub test_email_prefix: Option<String>,
    pub allowed_attachments: Option<String>,
    pub bounce_setup: Option<i16>,
    pub complaint_setup: Option<i16>,
    pub campaigns_only: Option<i16>,
    pub templates_only: Option<i16>,
    pub lists_only: Option<i16>,
    pub gdpr_only: Option<i16>,
    pub gdpr_only_ar: Option<i16>,
    pub gdpr_options: Option<i16>,
    pub recaptcha_sitekey: Option<String>,
    pub recaptcha_secretkey: Option<String>,
    pub custom_domain: Option<String>,
    pub custom_domain_protocol: Option<String>,
    pub custom_domain_enabled: Option<i16>,
    pub templates_lists_sorting: Option<String>,
    pub campaign_report_rows: Option<i32>,
    pub query_string: Option<String>,
    pub opens_tracking: Option<i16>,
    pub links_tracking: Option<i16>,
    pub opt_in: Option<i16>,
    pub hide_lists: Option<i16>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::login::Entity",
        from = "Column::UserId",
        to = "super::login::Column::Id"
    )]
    Login,
    #[sea_orm(has_many = "super::lists::Entity")]
    Lists,
}

impl Related<super::login::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Login.def()
    }
}

impl Related<super::lists::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Lists.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
