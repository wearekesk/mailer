//! Model Context Protocol (MCP) Server for Mailer automation.
//!
//! Uses `rmcp` with `#[tool]`, `#[tool_router]`, and `#[tool_handler]` macros
//! to define first-class AI tools that agents can discover and execute to inspect
//! and manage brands, email lists, subscribers, campaigns, templates, segments,
//! automation rules, suppression lists, and blocked domains.
//!
//! Handlers and parameter definitions are partitioned into modules:
//! - `brands`
//! - `lists`
//! - `subscribers`
//! - `campaigns`
//! - `templates`
//! - `segments`
//! - `rules`
//! - `settings`

pub mod brands;
pub mod campaigns;
pub mod lists;
pub mod rules;
pub mod segments;
pub mod settings;
pub mod subscribers;
pub mod templates;

use rmcp::{
    handler::server::{
        router::tool::ToolRouter,
        wrapper::{Json as McpJson, Parameters},
    },
    tool, tool_router, ErrorData,
};

use crate::routes::api::{
    brands::BrandResponse,
    campaigns::CreateCampaignOutcome,
    lists::ListResponse,
    rules::RuleResponse,
    segments::SegmentResponse,
    settings::{BlockedDomainResponse, SuppressionResponse},
    subscribers::{AckResponse, CountResponse, StatusResponse, SubscribeResponse},
    templates::TemplateResponse,
};
use crate::AppState;

// ── MCP Server Handler with Macros ───────────────────────────────────────────

#[derive(Clone)]
pub struct MailerMcpServer {
    state: AppState,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl MailerMcpServer {
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router(server_handler)]
impl MailerMcpServer {
    // ── Brands ───────────────────────────────────────────────────────────────

    #[tool(name = "list_brands", description = "List all configured email brands in Mailer.")]
    pub async fn list_brands(&self, params: Parameters<brands::EmptyParams>) -> Result<McpJson<Vec<BrandResponse>>, ErrorData> {
        brands::list_brands(&self.state, params).await
    }

    #[tool(name = "get_brand", description = "Get details of a specific brand.")]
    pub async fn get_brand(&self, params: Parameters<brands::BrandParam>) -> Result<McpJson<BrandResponse>, ErrorData> {
        brands::get_brand(&self.state, params).await
    }

    #[tool(name = "create_brand", description = "Create a new brand.")]
    pub async fn create_brand(&self, params: Parameters<brands::CreateBrandParams>) -> Result<McpJson<BrandResponse>, ErrorData> {
        brands::create_brand(&self.state, params).await
    }

    #[tool(name = "update_brand", description = "Update an existing brand.")]
    pub async fn update_brand(&self, params: Parameters<brands::UpdateBrandParams>) -> Result<McpJson<AckResponse>, ErrorData> {
        brands::update_brand(&self.state, params).await
    }

    #[tool(name = "delete_brand", description = "Delete a brand and all associated lists and subscribers.")]
    pub async fn delete_brand(&self, params: Parameters<brands::BrandParam>) -> Result<McpJson<AckResponse>, ErrorData> {
        brands::delete_brand(&self.state, params).await
    }

    // ── Lists ─────────────────────────────────────────────────────────────────

    #[tool(name = "list_lists_for_brand", description = "List all email lists belonging to a specific brand.")]
    pub async fn list_lists_for_brand(&self, params: Parameters<lists::ListForBrandParams>) -> Result<McpJson<Vec<ListResponse>>, ErrorData> {
        lists::list_lists_for_brand(&self.state, params).await
    }

    #[tool(name = "get_list", description = "Get details of a specific mailing list.")]
    pub async fn get_list(&self, params: Parameters<lists::ListParam>) -> Result<McpJson<ListResponse>, ErrorData> {
        lists::get_list(&self.state, params).await
    }

    #[tool(name = "create_list", description = "Create a new mailing list under a brand.")]
    pub async fn create_list(&self, params: Parameters<lists::CreateListParams>) -> Result<McpJson<ListResponse>, ErrorData> {
        lists::create_list(&self.state, params).await
    }

    #[tool(name = "update_list", description = "Update an existing mailing list.")]
    pub async fn update_list(&self, params: Parameters<lists::UpdateListParams>) -> Result<McpJson<AckResponse>, ErrorData> {
        lists::update_list(&self.state, params).await
    }

    #[tool(name = "delete_list", description = "Delete a mailing list and its subscribers.")]
    pub async fn delete_list(&self, params: Parameters<lists::ListParam>) -> Result<McpJson<AckResponse>, ErrorData> {
        lists::delete_list(&self.state, params).await
    }

    // ── Subscribers ───────────────────────────────────────────────────────────

    #[tool(name = "subscribe", description = "Subscribe or add an email address to a mailing list.")]
    pub async fn subscribe(&self, params: Parameters<subscribers::SubscribeParams>) -> Result<McpJson<SubscribeResponse>, ErrorData> {
        subscribers::subscribe(&self.state, params).await
    }

    #[tool(name = "get_subscriber_status", description = "Check subscription status of an email in a mailing list.")]
    pub async fn get_subscriber_status(&self, params: Parameters<subscribers::SubscriberListEmailParams>) -> Result<McpJson<StatusResponse>, ErrorData> {
        subscribers::get_subscriber_status(&self.state, params).await
    }

    #[tool(name = "get_subscriber_count", description = "Get count of active subscribers in a mailing list.")]
    pub async fn get_subscriber_count(&self, params: Parameters<subscribers::SubscriberCountParams>) -> Result<McpJson<CountResponse>, ErrorData> {
        subscribers::get_subscriber_count(&self.state, params).await
    }

    #[tool(name = "unsubscribe", description = "Unsubscribe an email address from a mailing list.")]
    pub async fn unsubscribe(&self, params: Parameters<subscribers::SubscriberListEmailParams>) -> Result<McpJson<AckResponse>, ErrorData> {
        subscribers::unsubscribe(&self.state, params).await
    }

    #[tool(name = "delete_subscriber", description = "Permanently delete/erase an email address from a mailing list (GDPR).")]
    pub async fn delete_subscriber(&self, params: Parameters<subscribers::SubscriberListEmailParams>) -> Result<McpJson<AckResponse>, ErrorData> {
        subscribers::delete_subscriber(&self.state, params).await
    }

    #[tool(name = "query_subscribers", description = "Query subscribers in a specific email list.")]
    pub async fn query_subscribers(&self, params: Parameters<subscribers::QuerySubscribersParams>) -> Result<McpJson<Vec<subscribers::SubscriberItem>>, ErrorData> {
        subscribers::query_subscribers(&self.state, params).await
    }

    // ── Campaigns ────────────────────────────────────────────────────────────

    #[tool(name = "create_campaign", description = "Create a draft, scheduled, or immediate email campaign.")]
    pub async fn create_campaign(&self, params: Parameters<crate::routes::api::campaigns::CreateCampaignBody>) -> Result<McpJson<CreateCampaignOutcome>, ErrorData> {
        campaigns::create_campaign(&self.state, params).await
    }

    #[tool(name = "list_campaigns", description = "List recent email campaigns, optionally filtered by brand ID.")]
    pub async fn list_campaigns(&self, params: Parameters<campaigns::ListCampaignsParams>) -> Result<McpJson<Vec<campaigns::CampaignItem>>, ErrorData> {
        campaigns::list_campaigns(&self.state, params).await
    }

    #[tool(name = "delete_campaign", description = "Delete an email campaign.")]
    pub async fn delete_campaign(&self, params: Parameters<campaigns::CampaignParam>) -> Result<McpJson<AckResponse>, ErrorData> {
        campaigns::delete_campaign(&self.state, params).await
    }

    #[tool(name = "get_campaign_report", description = "Get analytics and report data for a campaign.")]
    pub async fn get_campaign_report(&self, params: Parameters<campaigns::CampaignParam>) -> Result<McpJson<campaigns::CampaignReportSummary>, ErrorData> {
        campaigns::get_campaign_report(&self.state, params).await
    }

    // ── Templates ────────────────────────────────────────────────────────────

    #[tool(name = "list_templates", description = "List email templates for a brand.")]
    pub async fn list_templates(&self, params: Parameters<brands::BrandParam>) -> Result<McpJson<Vec<TemplateResponse>>, ErrorData> {
        templates::list_templates(&self.state, params).await
    }

    #[tool(name = "create_template", description = "Create a new email template.")]
    pub async fn create_template(&self, params: Parameters<templates::CreateTemplateParams>) -> Result<McpJson<TemplateResponse>, ErrorData> {
        templates::create_template(&self.state, params).await
    }

    #[tool(name = "update_template", description = "Update an existing email template.")]
    pub async fn update_template(&self, params: Parameters<templates::UpdateTemplateParams>) -> Result<McpJson<AckResponse>, ErrorData> {
        templates::update_template(&self.state, params).await
    }

    #[tool(name = "delete_template", description = "Delete an email template.")]
    pub async fn delete_template(&self, params: Parameters<templates::TemplateParam>) -> Result<McpJson<AckResponse>, ErrorData> {
        templates::delete_template(&self.state, params).await
    }

    // ── Segments ─────────────────────────────────────────────────────────────

    #[tool(name = "list_segments", description = "List dynamic segments for a brand.")]
    pub async fn list_segments(&self, params: Parameters<brands::BrandParam>) -> Result<McpJson<Vec<SegmentResponse>>, ErrorData> {
        segments::list_segments(&self.state, params).await
    }

    #[tool(name = "create_segment", description = "Create a new segment on a mailing list.")]
    pub async fn create_segment(&self, params: Parameters<segments::CreateSegmentParams>) -> Result<McpJson<SegmentResponse>, ErrorData> {
        segments::create_segment(&self.state, params).await
    }

    #[tool(name = "delete_segment", description = "Delete a segment.")]
    pub async fn delete_segment(&self, params: Parameters<segments::SegmentParam>) -> Result<McpJson<AckResponse>, ErrorData> {
        segments::delete_segment(&self.state, params).await
    }

    // ── Automation Rules ─────────────────────────────────────────────────────

    #[tool(name = "list_rules", description = "List automation rules for a brand.")]
    pub async fn list_rules(&self, params: Parameters<brands::BrandParam>) -> Result<McpJson<Vec<RuleResponse>>, ErrorData> {
        rules::list_rules(&self.state, params).await
    }

    #[tool(name = "create_rule", description = "Create an automation rule for a brand.")]
    pub async fn create_rule(&self, params: Parameters<rules::CreateRuleParams>) -> Result<McpJson<RuleResponse>, ErrorData> {
        rules::create_rule(&self.state, params).await
    }

    #[tool(name = "toggle_rule", description = "Toggle an automation rule on or off.")]
    pub async fn toggle_rule(&self, params: Parameters<rules::RuleParam>) -> Result<McpJson<AckResponse>, ErrorData> {
        rules::toggle_rule(&self.state, params).await
    }

    #[tool(name = "delete_rule", description = "Delete an automation rule.")]
    pub async fn delete_rule(&self, params: Parameters<rules::RuleParam>) -> Result<McpJson<AckResponse>, ErrorData> {
        rules::delete_rule(&self.state, params).await
    }

    // ── Suppression Lists & Blocked Domains ──────────────────────────────────

    #[tool(name = "list_suppression", description = "List suppressed email addresses for a brand.")]
    pub async fn list_suppression(&self, params: Parameters<brands::BrandParam>) -> Result<McpJson<Vec<SuppressionResponse>>, ErrorData> {
        settings::list_suppression(&self.state, params).await
    }

    #[tool(name = "add_suppression", description = "Add an email address to the brand suppression list.")]
    pub async fn add_suppression(&self, params: Parameters<settings::AddSuppressionParams>) -> Result<McpJson<AckResponse>, ErrorData> {
        settings::add_suppression(&self.state, params).await
    }

    #[tool(name = "delete_suppression", description = "Remove an entry from the suppression list by ID.")]
    pub async fn delete_suppression(&self, params: Parameters<settings::SuppressionParam>) -> Result<McpJson<AckResponse>, ErrorData> {
        settings::delete_suppression(&self.state, params).await
    }

    #[tool(name = "list_blocked_domains", description = "List blocked email domains for a brand.")]
    pub async fn list_blocked_domains(&self, params: Parameters<brands::BrandParam>) -> Result<McpJson<Vec<BlockedDomainResponse>>, ErrorData> {
        settings::list_blocked_domains(&self.state, params).await
    }

    #[tool(name = "add_blocked_domain", description = "Add a domain to the brand's blocked domains list.")]
    pub async fn add_blocked_domain(&self, params: Parameters<settings::AddBlockedDomainParams>) -> Result<McpJson<AckResponse>, ErrorData> {
        settings::add_blocked_domain(&self.state, params).await
    }

    #[tool(name = "delete_blocked_domain", description = "Remove a blocked domain by ID.")]
    pub async fn delete_blocked_domain(&self, params: Parameters<settings::BlockedDomainParam>) -> Result<McpJson<AckResponse>, ErrorData> {
        settings::delete_blocked_domain(&self.state, params).await
    }
}

// ── Streamable HTTP Service for MCP ──────────────────────────────────────────

use std::sync::Arc;
use poem::{endpoint::TowerCompatExt, EndpointExt};
use rmcp::transport::StreamableHttpService;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager,
    StreamableHttpServerConfig,
};

/// Creates a streamable HTTP service for MCP handling tool discovery, execution,
/// SSE transport, and session management without manual JSON-RPC dispatching.
pub fn create_mcp_service(
    state: AppState,
) -> StreamableHttpService<MailerMcpServer, LocalSessionManager> {
    StreamableHttpService::new(
        move || Ok(MailerMcpServer::new(state.clone())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    )
}

/// Creates a Poem endpoint for MCP, wrapped in tower-compat and secured by API key authentication.
pub fn mcp_endpoint(state: AppState) -> impl poem::Endpoint {
    create_mcp_service(state)
        .compat()
        .with(crate::routes::api::auth::ApiKeyAuthMiddleware)
}
