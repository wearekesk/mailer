use sea_orm_migration::prelude::*;

mod m20240101_000001_login;
mod m20240101_000002_apps;
mod m20240101_000003_lists;
mod m20240101_000004_subscribers;
mod m20240101_000005_campaigns;
mod m20240101_000006_links;
mod m20240101_000007_queue;
mod m20240101_000008_ares;
mod m20240101_000009_ares_emails;
mod m20240101_000010_template;
mod m20240101_000011_zapier;
mod m20240101_000012_seg;
mod m20240101_000013_seg_cons;
mod m20240101_000014_subscribers_seg;
mod m20240101_000015_suppression_list;
mod m20240101_000016_blocked_domains;
mod m20240101_000017_skipped_emails;
mod m20240101_000018_rules;
mod m20240101_000019_webhooks_log;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_login::Migration),
            Box::new(m20240101_000002_apps::Migration),
            Box::new(m20240101_000003_lists::Migration),
            Box::new(m20240101_000004_subscribers::Migration),
            Box::new(m20240101_000005_campaigns::Migration),
            Box::new(m20240101_000006_links::Migration),
            Box::new(m20240101_000007_queue::Migration),
            Box::new(m20240101_000008_ares::Migration),
            Box::new(m20240101_000009_ares_emails::Migration),
            Box::new(m20240101_000010_template::Migration),
            Box::new(m20240101_000011_zapier::Migration),
            Box::new(m20240101_000012_seg::Migration),
            Box::new(m20240101_000013_seg_cons::Migration),
            Box::new(m20240101_000014_subscribers_seg::Migration),
            Box::new(m20240101_000015_suppression_list::Migration),
            Box::new(m20240101_000016_blocked_domains::Migration),
            Box::new(m20240101_000017_skipped_emails::Migration),
            Box::new(m20240101_000018_rules::Migration),
            Box::new(m20240101_000019_webhooks_log::Migration),
        ]
    }
}
