use std::sync::Arc;

use poem::{listener::TcpListener, Server};
use sea_orm::Database;
use sea_orm_migration::MigratorTrait;

use mailer::database::migrations::Migrator;
use mailer::utils::config::Config;
use mailer::{routes, services, utils, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    // Default log level: app code at INFO, sqlx::query at WARN so the
    // per-statement SQL flood is muted. RUST_LOG overrides everything if
    // set (e.g. `RUST_LOG=mailer=debug,sqlx=debug` to see queries again).
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,sqlx::query=warn")),
        )
        .init();

    let config = Config::from_env()?;
    utils::crypto::init_secret(&config.secret_key);
    services::geo::init(None);
    let db = Database::connect(&config.database_url).await?;

    tracing::info!("Running database migrations...");
    Migrator::up(&db, None).await?;

    tracing::info!("Ensuring database is seeded...");
    services::seed::seed_if_needed(&db).await?;

    let state = AppState {
        db: db.clone(),
        config: Arc::new(config.clone()),
    };

    // Spawn the campaign scheduler in the background. Uses the StubMailer
    // until SMTP/SES is configured — replace with a real provider later.
    let mailer: Arc<dyn services::mailer::EmailProvider> = Arc::new(services::mailer::StubMailer);
    services::scheduler::spawn(db, mailer);

    let app = routes::build(state);

    tracing::info!("Listening on {}", config.bind_addr);
    Server::new(TcpListener::bind(&config.bind_addr))
        .run(app)
        .await?;

    Ok(())
}
