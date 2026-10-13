//! Automated database seeding on application startup.
//!
//! Creates the initial admin login row (from `MAILER_ADMIN_EMAIL` /
//! `MAILER_ADMIN_PASSWORD` or fallback `SENDY_ADMIN_EMAIL`), plus
//! demo brands and lists so the dashboard isn't empty on a clean installation.
//!
//! Idempotent: on subsequent runs with the same email, the password is
//! maintained or updated if set. Demo brands and lists are upserted by
//! name so running startup seeding never duplicates rows.

use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    QueryFilter,
};

use crate::database::entities::{apps, lists, login};
use crate::services::auth;

/// Checks and seeds the database with the default admin user and demo brands.
pub async fn seed_if_needed(db: &DatabaseConnection) -> anyhow::Result<()> {
    let email = std::env::var("MAILER_ADMIN_EMAIL")
        .or_else(|_| std::env::var("SENDY_ADMIN_EMAIL"))
        .unwrap_or_else(|_| "admin@mailer.local".into());
    let password = std::env::var("MAILER_ADMIN_PASSWORD")
        .or_else(|_| std::env::var("SENDY_ADMIN_PASSWORD"))
        .unwrap_or_else(|_| "ChangeMe!2026".into());

    let hashed = auth::hash_password(&password)?;
    let api_key = random_alphanumeric(40);
    let auth_salt = random_alphanumeric(16);

    let existing: Option<login::Model> = login::Entity::find()
        .filter(login::Column::Username.eq(&email))
        .one(db)
        .await?;

    let user_id = match existing {
        Some(row) => {
            let id = row.id;
            tracing::info!(user_id = id, %email, "Admin user verified");
            id
        }
        None => {
            let row = login::ActiveModel {
                username: Set(Some(email.clone())),
                password: Set(Some(hashed)),
                api_key: Set(Some(api_key.clone())),
                auth_salt: Set(Some(auth_salt)),
                name: Set(Some("Admin".to_string())),
                company: Set(Some("Mailer".to_string())),
                language: Set(Some("en".to_string())),
                timezone: Set(Some("UTC".to_string())),
                cron: Set(Some(0)),
                ..Default::default()
            };
            let inserted = row.insert(db).await?;
            tracing::info!(user_id = inserted.id, %email, "Created admin user");
            inserted.id
        }
    };

    let demo_brands = seed_demo_brands(db, user_id).await?;
    tracing::info!(admin_email = %email, brands = ?demo_brands, "Database seeding complete");
    Ok(())
}

struct DemoBrand {
    name: &'static str,
    from_name: &'static str,
    from_email: &'static str,
    reply_to: &'static str,
    lists: &'static [&'static str],
}

const DEMO_BRANDS: &[DemoBrand] = &[
    DemoBrand {
        name: "Acme Newsletter",
        from_name: "Acme Inc",
        from_email: "newsletter@acme.example",
        reply_to: "support@acme.example",
        lists: &["Weekly digest", "Product updates", "Customer announcements"],
    },
    DemoBrand {
        name: "Globex Marketing",
        from_name: "Globex",
        from_email: "hello@globex.example",
        reply_to: "hello@globex.example",
        lists: &["Promotions", "Beta program"],
    },
];

async fn seed_demo_brands(
    db: &DatabaseConnection,
    user_id: i32,
) -> anyhow::Result<Vec<String>> {
    let mut names = Vec::with_capacity(DEMO_BRANDS.len());
    for spec in DEMO_BRANDS {
        let existing: Option<apps::Model> = apps::Entity::find()
            .filter(apps::Column::AppName.eq(spec.name))
            .one(db)
            .await?;

        let brand_id = match existing {
            Some(b) => b.id,
            None => {
                let app_key = random_alphanumeric(32);
                let new = apps::ActiveModel {
                    user_id: Set(Some(user_id)),
                    app_name: Set(Some(spec.name.to_string())),
                    from_name: Set(Some(spec.from_name.to_string())),
                    from_email: Set(Some(spec.from_email.to_string())),
                    reply_to: Set(Some(spec.reply_to.to_string())),
                    app_key: Set(Some(app_key)),
                    opens_tracking: Set(Some(1)),
                    links_tracking: Set(Some(1)),
                    // Default to local Mailpit in development
                    smtp_host: Set(Some("localhost".to_string())),
                    smtp_port: Set(Some("1025".to_string())),
                    ..Default::default()
                };
                let inserted = new.insert(db).await?;
                tracing::info!(brand_id = inserted.id, name = spec.name, "created demo brand");
                inserted.id
            }
        };

        for list_name in spec.lists {
            let exists: Option<lists::Model> = lists::Entity::find()
                .filter(lists::Column::App.eq(brand_id))
                .filter(lists::Column::Name.eq(*list_name))
                .one(db)
                .await?;
            if exists.is_some() {
                continue;
            }
            lists::ActiveModel {
                app: Set(Some(brand_id)),
                user_id: Set(Some(user_id)),
                name: Set(Some(list_name.to_string())),
                opt_in: Set(Some(0)),
                hide: Set(Some(0)),
                ..Default::default()
            }
            .insert(db)
            .await?;
            tracing::info!(brand_id, list = list_name, "created demo list");
        }

        names.push(spec.name.to_string());
    }
    Ok(names)
}

fn random_alphanumeric(len: usize) -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}
