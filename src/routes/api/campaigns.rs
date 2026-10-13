use poem::{
    handler,
    web::{Data, Json, Path, Query},
    IntoResponse, Response,
};
use schemars::JsonSchema;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

use crate::database::entities::campaigns;
use crate::routes::api::{auth::ApiAuth, bad_request, internal_error, not_found};
use crate::services::{campaigns as campaigns_svc, reports};
use crate::utils::opaque_id::{Brand, Campaign, List, OpaqueId, Segment};
use crate::AppState;

#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
pub struct CampaignQuery {
    #[serde(default)]
    pub brand_id: Option<OpaqueId<Brand>>,
    #[serde(default)]
    pub page: Option<u64>,
    #[serde(default)]
    pub limit: Option<u64>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CampaignSummaryResponse {
    pub id: OpaqueId<Campaign>,
    pub brand_id: Option<OpaqueId<Brand>>,
    pub title: Option<String>,
    pub from_name: Option<String>,
    pub from_email: Option<String>,
    pub subject: Option<String>,
    pub recipients: Option<i32>,
    pub to_send: Option<i32>,
    pub sent: Option<String>,
}

/// JSON body for `POST /api/campaigns`.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
pub struct CreateCampaignBody {
    pub from_name: String,
    pub from_email: String,
    pub reply_to: String,
    pub subject: String,
    pub html_text: String,

    #[serde(default)]
    pub plain_text: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub query_string: Option<String>,
    /// Required when saving a draft. Derived from list/segment ids when sending.
    #[serde(default)]
    pub brand_id: Option<OpaqueId<Brand>>,

    #[serde(default)]
    pub list_ids: Vec<OpaqueId<List>>,
    #[serde(default)]
    pub segment_ids: Vec<OpaqueId<Segment>>,
    #[serde(default)]
    pub exclude_list_ids: Vec<OpaqueId<List>>,
    #[serde(default, alias = "exclude_segments_ids")]
    pub exclude_segment_ids: Vec<OpaqueId<Segment>>,

    #[serde(default = "default_track")]
    pub track_opens: i16,
    #[serde(default = "default_track")]
    pub track_clicks: i16,

    #[serde(default)]
    pub send_campaign: bool,

    /// ISO-8601 timestamp string, an integer epoch, or `"YYYY-MM-DD HH:MM:SS"`.
    #[serde(default)]
    pub schedule_date_time: Option<String>,
    #[serde(default)]
    pub schedule_timezone: Option<String>,
}

fn default_track() -> i16 {
    1
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum CreateCampaignOutcome {
    Draft { campaign_id: OpaqueId<Campaign> },
    Scheduled { campaign_id: OpaqueId<Campaign> },
    Sending { campaign_id: OpaqueId<Campaign> },
}

impl From<campaigns_svc::CampaignOutcome> for CreateCampaignOutcome {
    fn from(o: campaigns_svc::CampaignOutcome) -> Self {
        match o {
            campaigns_svc::CampaignOutcome::Draft { campaign_id } => {
                CreateCampaignOutcome::Draft {
                    campaign_id: OpaqueId::new(campaign_id),
                }
            }
            campaigns_svc::CampaignOutcome::Scheduled { campaign_id } => {
                CreateCampaignOutcome::Scheduled {
                    campaign_id: OpaqueId::new(campaign_id),
                }
            }
            campaigns_svc::CampaignOutcome::Sending { campaign_id } => {
                CreateCampaignOutcome::Sending {
                    campaign_id: OpaqueId::new(campaign_id),
                }
            }
        }
    }
}

/// `GET /api/campaigns` — list campaigns with optional brand filter and pagination.
#[handler]
pub async fn list(
    _auth: ApiAuth,
    Query(q): Query<CampaignQuery>,
    Data(state): Data<&AppState>,
) -> Response {
    let mut query = campaigns::Entity::find().order_by_desc(campaigns::Column::Id);
    if let Some(ref bid) = q.brand_id {
        query = query.filter(campaigns::Column::App.eq(bid.raw()));
    }

    let page = q.page.unwrap_or(0);
    let limit = q.limit.unwrap_or(50).min(100);

    let paginator = query.paginate(&state.db, limit);
    match paginator.fetch_page(page).await {
        Ok(rows) => {
            let body: Vec<CampaignSummaryResponse> = rows
                .into_iter()
                .map(|c| CampaignSummaryResponse {
                    id: OpaqueId::new(c.id),
                    brand_id: c.app.map(OpaqueId::new),
                    title: c.title,
                    from_name: c.from_name,
                    from_email: c.from_email,
                    subject: c.subject,
                    recipients: c.recipients,
                    to_send: c.to_send,
                    sent: c.sent,
                })
                .collect();
            Json(body).into_response()
        }
        Err(e) => {
            tracing::error!(error = ?e, "list campaigns failed");
            internal_error("Database error.")
        }
    }
}

/// `GET /api/campaigns/:campaign_id` — fetch campaign details.
#[handler]
pub async fn get(
    _auth: ApiAuth,
    Path(campaign_id): Path<OpaqueId<Campaign>>,
    Data(state): Data<&AppState>,
) -> Response {
    match campaigns::Entity::find_by_id(campaign_id.raw()).one(&state.db).await {
        Ok(Some(c)) => {
            let resp = CampaignSummaryResponse {
                id: OpaqueId::new(c.id),
                brand_id: c.app.map(OpaqueId::new),
                title: c.title,
                from_name: c.from_name,
                from_email: c.from_email,
                subject: c.subject,
                recipients: c.recipients,
                to_send: c.to_send,
                sent: c.sent,
            };
            Json(resp).into_response()
        }
        Ok(None) => not_found("Campaign does not exist."),
        Err(e) => {
            tracing::error!(error = ?e, "find campaign failed");
            internal_error("Database error.")
        }
    }
}

/// `POST /api/campaigns` — create a draft, schedule, or immediate send.
#[handler]
pub async fn create(
    auth: ApiAuth,
    Json(body): Json<CreateCampaignBody>,
    Data(state): Data<&AppState>,
) -> Response {
    if body.from_name.trim().is_empty() {
        return bad_request("from_name is required");
    }
    if body.from_email.trim().is_empty() {
        return bad_request("from_email is required");
    }
    if body.reply_to.trim().is_empty() {
        return bad_request("reply_to is required");
    }
    if body.subject.trim().is_empty() {
        return bad_request("subject is required");
    }
    if body.html_text.trim().is_empty() {
        return bad_request("html_text is required");
    }

    let schedule_date_time = match body.schedule_date_time.as_deref().map(parse_schedule) {
        Some(Some(t)) => Some(t),
        Some(None) => return bad_request("schedule_date_time is invalid"),
        None => None,
    };

    if body.send_campaign && body.list_ids.is_empty() && body.segment_ids.is_empty() {
        return bad_request("list_ids or segment_ids required when send_campaign is true");
    }
    if !body.send_campaign && schedule_date_time.is_none() && body.brand_id.is_none() {
        return bad_request("brand_id is required for drafts");
    }

    let create_params = campaigns_svc::CreateCampaignParams {
        user_id: auth.0.id,
        brand_id: body.brand_id.map(|b| b.raw()),
        from_name: body.from_name,
        from_email: body.from_email,
        reply_to: body.reply_to,
        title: body.title,
        subject: body.subject,
        plain_text: body.plain_text,
        html_text: body.html_text,
        list_ids: body.list_ids.into_iter().map(|i| i.raw()).collect(),
        segment_ids: body.segment_ids.into_iter().map(|i| i.raw()).collect(),
        exclude_list_ids: body.exclude_list_ids.into_iter().map(|i| i.raw()).collect(),
        exclude_segment_ids: body
            .exclude_segment_ids
            .into_iter()
            .map(|i| i.raw())
            .collect(),
        track_opens: clamped_track(body.track_opens),
        track_clicks: clamped_track(body.track_clicks),
        send_campaign: body.send_campaign,
        schedule_date_time,
        schedule_timezone: body.schedule_timezone,
        query_string: body.query_string,
    };

    match campaigns_svc::create(&state.db, create_params).await {
        Ok(outcome) => Json(CreateCampaignOutcome::from(outcome)).into_response(),
        Err(crate::utils::errors::AppError::BadRequest(msg)) => bad_request(msg),
        Err(e) => {
            tracing::error!(error = ?e, "create campaign failed");
            internal_error("Unable to create campaign")
        }
    }
}

/// `DELETE /api/campaigns/:campaign_id` — delete campaign.
#[handler]
pub async fn delete(
    _auth: ApiAuth,
    Path(campaign_id): Path<OpaqueId<Campaign>>,
    Data(state): Data<&AppState>,
) -> Response {
    let raw = campaign_id.raw();
    match campaigns::Entity::delete_by_id(raw).exec(&state.db).await {
        Ok(res) if res.rows_affected > 0 => Json(serde_json::json!({ "ok": true })).into_response(),
        Ok(_) => not_found("Campaign does not exist."),
        Err(e) => {
            tracing::error!(error = ?e, "delete campaign failed");
            internal_error("Failed to delete campaign.")
        }
    }
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CampaignReportResponse {
    pub recipients: i64,
    pub to_send: i64,
    pub unique_opens: i64,
    pub total_opens: i64,
    pub open_rate: f64,
    pub unique_clickers: i64,
    pub total_clicks: i64,
    pub click_rate: f64,
    pub unsubscribed: i64,
    pub unsubscribe_rate: f64,
    pub bounced: i64,
    pub bounce_rate: f64,
    pub complained: i64,
}

/// `GET /api/campaigns/:campaign_id/report` — fetch performance report.
#[handler]
pub async fn report(
    _auth: ApiAuth,
    Path(campaign_id): Path<OpaqueId<Campaign>>,
    Data(state): Data<&AppState>,
) -> Response {
    match reports::build(&state.db, campaign_id.raw()).await {
        Ok(r) => {
            let resp = CampaignReportResponse {
                recipients: r.recipients,
                to_send: r.to_send,
                unique_opens: r.unique_opens,
                total_opens: r.total_opens,
                open_rate: r.open_rate(),
                unique_clickers: r.unique_clickers,
                total_clicks: r.total_clicks,
                click_rate: r.click_rate(),
                unsubscribed: r.unsubscribed,
                unsubscribe_rate: r.unsubscribe_rate(),
                bounced: r.bounced,
                bounce_rate: r.bounce_rate(),
                complained: r.complained,
            };
            Json(resp).into_response()
        }
        Err(e) => {
            tracing::error!(error = ?e, "build report failed");
            internal_error("Failed to generate campaign report.")
        }
    }
}

pub(crate) fn clamped_track(value: i16) -> i16 {
    if (0..=2).contains(&value) {
        value
    } else {
        1
    }
}

pub(crate) fn parse_schedule(value: &str) -> Option<i64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Ok(n) = value.parse::<i64>() {
        return Some(n);
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(value) {
        return Some(dt.timestamp());
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return Some(naive.and_utc().timestamp());
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S") {
        return Some(naive.and_utc().timestamp());
    }
    None
}
