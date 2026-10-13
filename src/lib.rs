#![recursion_limit = "512"]
//! Shared modules for Mailer — self-hosted email delivery and marketing.

pub mod components;
pub mod database;
pub mod mcp;
pub mod routes;
pub mod services;
pub mod utils;

use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: sea_orm::DatabaseConnection,
    pub config: Arc<utils::config::Config>,
}
