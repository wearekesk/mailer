use rmcp::{
    handler::server::wrapper::{Json as McpJson, Parameters},
    schemars::JsonSchema,
    ErrorData,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

use crate::database::entities::campaigns;
use crate::routes::api::{
    campaigns::{clamped_track, parse_schedule, CreateCampaignBody, CreateCampaignOutcome},
    subscribers::AckResponse,
};
use crate::services::{campaigns as campaigns_svc, reports as reports_svc};
use crate::utils::opaque_id::{Campaign, OpaqueId};
use crate::AppState;

#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
pub struct ListCampaignsParams {
    #[schemars(description = "Optional brand ID filter")]
    pub brand_id: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct CampaignParam {
    #[schemars(description = "Opaque identifier of the campaign")]
    pub campaign_id: OpaqueId<Campaign>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CampaignItem {
    pub id: i32,
    pub title: String,
    pub subject: String,
    pub recipients: Option<i32>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CampaignReportSummary {
    pub recipients: i64,
    pub to_send: i64,
    pub unique_opens: i64,
    pub total_opens: i64,
    pub open_rate_pct: f64,
    pub unique_clickers: i64,
    pub total_clicks: i64,
    pub click_rate_pct: f64,
    pub unsubscribed: i64,
    pub unsubscribe_rate_pct: f64,
    pub bounced: i64,
    pub bounce_rate_pct: f64,
    pub complained: i64,
}

pub async fn create_campaign(
    state: &AppState,
    Parameters(body): Parameters<CreateCampaignBody>,
) -> Result<McpJson<CreateCampaignOutcome>, ErrorData> {
    if body.from_name.trim().is_empty() {
        return Err(ErrorData::invalid_params("from_name is required", None));
    }
    if body.from_email.trim().is_empty() {
        return Err(ErrorData::invalid_params("from_email is required", None));
    }
    if body.reply_to.trim().is_empty() {
        return Err(ErrorData::invalid_params("reply_to is required", None));
    }
    if body.subject.trim().is_empty() {
        return Err(ErrorData::invalid_params("subject is required", None));
    }
    if body.html_text.trim().is_empty() {
        return Err(ErrorData::invalid_params("html_text is required", None));
    }

    let schedule_date_time = match body.schedule_date_time.as_deref().map(parse_schedule) {
        Some(Some(t)) => Some(t),
        Some(None) => return Err(ErrorData::invalid_params("schedule_date_time is invalid", None)),
        None => None,
    };

    if body.send_campaign && body.list_ids.is_empty() && body.segment_ids.is_empty() {
        return Err(ErrorData::invalid_params(
            "list_ids or segment_ids required when send_campaign is true",
            None,
        ));
    }
    if !body.send_campaign && schedule_date_time.is_none() && body.brand_id.is_none() {
        return Err(ErrorData::invalid_params(
            "brand_id is required for drafts",
            None,
        ));
    }

    let create_params = campaigns_svc::CreateCampaignParams {
        user_id: 1, // Default admin user ID for automated MCP requests
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
        exclude_segment_ids: body.exclude_segment_ids.into_iter().map(|i| i.raw()).collect(),
        track_opens: clamped_track(body.track_opens),
        track_clicks: clamped_track(body.track_clicks),
        send_campaign: body.send_campaign,
        schedule_date_time,
        schedule_timezone: body.schedule_timezone,
        query_string: body.query_string,
    };

    let outcome = campaigns_svc::create(&state.db, create_params)
        .await
        .map_err(|e| ErrorData::internal_error(format!("campaigns::create error: {e}"), None))?;

    Ok(McpJson(CreateCampaignOutcome::from(outcome)))
}

pub async fn list_campaigns(
    state: &AppState,
    Parameters(params): Parameters<ListCampaignsParams>,
) -> Result<McpJson<Vec<CampaignItem>>, ErrorData> {
    let mut query = campaigns::Entity::find().order_by_desc(campaigns::Column::Id);
    if let Some(bid) = params.brand_id {
        query = query.filter(campaigns::Column::App.eq(bid));
    }

    let rows = query
        .all(&state.db)
        .await
        .map_err(|e| ErrorData::internal_error(format!("db error: {e}"), None))?;

    let items = rows
        .into_iter()
        .take(50)
        .map(|r| CampaignItem {
            id: r.id,
            title: r.title.unwrap_or_default(),
            subject: r.subject.unwrap_or_default(),
            recipients: r.recipients,
        })
        .collect();

    Ok(McpJson(items))
}

pub async fn delete_campaign(
    state: &AppState,
    Parameters(params): Parameters<CampaignParam>,
) -> Result<McpJson<AckResponse>, ErrorData> {
    campaigns::Entity::delete_by_id(params.campaign_id.raw())
        .exec(&state.db)
        .await
        .map_err(|e| ErrorData::internal_error(format!("delete_campaign error: {e}"), None))?;
    Ok(McpJson(AckResponse { ok: true }))
}

pub async fn get_campaign_report(
    state: &AppState,
    Parameters(params): Parameters<CampaignParam>,
) -> Result<McpJson<CampaignReportSummary>, ErrorData> {
    let r = reports_svc::build(&state.db, params.campaign_id.raw())
        .await
        .map_err(|e| ErrorData::internal_error(format!("build report error: {e}"), None))?;

    Ok(McpJson(CampaignReportSummary {
        recipients: r.recipients,
        to_send: r.to_send,
        unique_opens: r.unique_opens,
        total_opens: r.total_opens,
        open_rate_pct: r.open_rate(),
        unique_clickers: r.unique_clickers,
        total_clicks: r.total_clicks,
        click_rate_pct: r.click_rate(),
        unsubscribed: r.unsubscribed,
        unsubscribe_rate_pct: r.unsubscribe_rate(),
        bounced: r.bounced,
        bounce_rate_pct: r.bounce_rate(),
        complained: r.complained,
    }))
}
