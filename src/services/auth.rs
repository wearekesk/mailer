//! Authentication helpers — login, password hashing (argon2), TOTP 2FA,
//! API-key lookup, password reset.
//!
//! Password storage: argon2id, encoded as PHC strings in `login.password`.
//! TOTP secrets: base32 in `login.auth_key`, validated with `totp-rs`.

use argon2::password_hash::{rand_core::OsRng, PasswordHash, SaltString};
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder,
};
use totp_rs::{Algorithm, Secret, TOTP};

use crate::database::entities::login;
use crate::utils::errors::{AppError, AppResult};

// ── API key (already used by the JSON API) ───────────────────────────────────

/// Look up the user that owns `api_key`. Returns `None` if the key is unknown.
pub async fn lookup_user_by_api_key(
    db: &DatabaseConnection,
    api_key: &str,
) -> AppResult<Option<login::Model>> {
    let user = login::Entity::find()
        .filter(login::Column::ApiKey.eq(api_key))
        .one(db)
        .await?;
    Ok(user)
}

/// Returns true if `api_key` matches a row in the `login` table.
pub async fn verify_api_key(db: &DatabaseConnection, api_key: &str) -> AppResult<bool> {
    let count = login::Entity::find()
        .filter(login::Column::ApiKey.eq(api_key))
        .count(db)
        .await?;
    Ok(count > 0)
}

// ── Password hashing (argon2id) ──────────────────────────────────────────────

/// Hash `plaintext` using argon2id with a random salt. Stores the result as
/// a PHC-format string suitable for round-trip with [`verify_password`].
pub fn hash_password(plaintext: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(plaintext.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("argon2 hash failed: {e}")))?;
    Ok(hash.to_string())
}

/// Verify `plaintext` against a stored argon2 PHC string. Returns `Ok(false)`
/// for a malformed stored hash or a wrong password (never panics).
pub fn verify_password(plaintext: &str, stored: &str) -> bool {
    let parsed = match PasswordHash::new(stored) {
        Ok(p) => p,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(plaintext.as_bytes(), &parsed)
        .is_ok()
}

// ── Login (username + password) ──────────────────────────────────────────────

/// Outcome of a [`login`] attempt.
pub enum LoginOutcome {
    /// Authentication failed — invalid email or password.
    Invalid,
    /// Authentication succeeded; caller may issue a session cookie.
    Authenticated(login::Model),
    /// Authentication succeeded but the account requires a TOTP code next.
    /// Caller should issue a short-lived "pre-2fa" cookie containing the
    /// returned `user_id` and redirect to the 2FA form.
    NeedsTwoFactor(login::Model),
}

/// Verify a username + password pair. The `login` table column is
/// called `username` but contains an email address.
pub async fn login(
    db: &DatabaseConnection,
    username: &str,
    plaintext_password: &str,
) -> AppResult<LoginOutcome> {
    let user = login::Entity::find()
        .filter(login::Column::Username.eq(username))
        .one(db)
        .await?;

    let user = match user {
        Some(u) => u,
        None => return Ok(LoginOutcome::Invalid),
    };

    let stored = user.password.as_deref().unwrap_or("");
    if !verify_password(plaintext_password, stored) {
        return Ok(LoginOutcome::Invalid);
    }

    // Clear any pending password-reset key on a successful login.
    if user.reset_password_key.as_deref().unwrap_or("").is_empty() == false {
        let mut active: login::ActiveModel = user.clone().into();
        active.reset_password_key = Set(Some(String::new()));
        active.update(db).await?;
    }

    if user.auth_enabled.unwrap_or(0) == 1 && !user.auth_key.as_deref().unwrap_or("").is_empty() {
        Ok(LoginOutcome::NeedsTwoFactor(user))
    } else {
        Ok(LoginOutcome::Authenticated(user))
    }
}

/// Look up a user by id — used by the session middleware to materialise a
/// row from a session cookie's user-id payload.
pub async fn find_user(
    db: &DatabaseConnection,
    user_id: i32,
) -> AppResult<Option<login::Model>> {
    Ok(login::Entity::find_by_id(user_id).one(db).await?)
}

// ── Two-factor authentication (TOTP) ─────────────────────────────────────────

/// Verify a 6-digit TOTP code against the user's `login.auth_key` secret.
pub fn verify_totp(secret_base32: &str, code: &str) -> bool {
    let Ok(secret) = Secret::Encoded(secret_base32.to_string()).to_bytes() else {
        return false;
    };
    let totp = TOTP::new_unchecked(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret,
        Some("Mailer".to_string()),
        "user".to_string(),
    );
    totp.check_current(code).unwrap_or(false)
}

/// Generate a fresh base32 TOTP secret for new 2FA enrolments.
pub fn generate_totp_secret() -> String {
    use rand::RngCore;
    let mut buf = [0u8; 20];
    rand::thread_rng().fill_bytes(&mut buf);
    Secret::Raw(buf.to_vec()).to_encoded().to_string()
}

/// Build an `otpauth://` URL for QR-code enrolment.
pub fn totp_provisioning_url(secret_base32: &str, account: &str, issuer: &str) -> Option<String> {
    let secret = Secret::Encoded(secret_base32.to_string()).to_bytes().ok()?;
    let totp = TOTP::new_unchecked(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret,
        Some(issuer.to_string()),
        account.to_string(),
    );
    Some(totp.get_url())
}

/// Generate an SVG QR code for an `otpauth://` URL. Returned string is a
/// complete `<svg>` element ready to drop into HTML.
pub fn qr_svg(otpauth_url: &str) -> Option<String> {
    use qrcode::render::svg;
    let code = qrcode::QrCode::new(otpauth_url.as_bytes()).ok()?;
    let svg = code
        .render::<svg::Color>()
        .min_dimensions(220, 220)
        .light_color(svg::Color("#ffffff"))
        .dark_color(svg::Color("#0f172a"))
        .build();
    Some(svg)
}

/// Persist a fresh secret on the user row (`auth_key`) but leave 2FA
/// disabled until the user verifies a code from it. Used by the enrolment
/// flow.
pub async fn begin_totp_enrollment(
    db: &DatabaseConnection,
    user_id: i32,
) -> AppResult<String> {
    let row = login::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("user {user_id}")))?;
    let secret = generate_totp_secret();
    let mut active: login::ActiveModel = row.into();
    active.auth_key = Set(Some(secret.clone()));
    active.auth_enabled = Set(Some(0));
    active.update(db).await?;
    Ok(secret)
}

/// Verify the user-supplied code against the pending secret. On success,
/// flip `auth_enabled = 1`.
pub async fn confirm_totp_enrollment(
    db: &DatabaseConnection,
    user_id: i32,
    code: &str,
) -> AppResult<bool> {
    let row = login::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("user {user_id}")))?;
    let secret = row.auth_key.clone().unwrap_or_default();
    if !verify_totp(&secret, code) {
        return Ok(false);
    }
    let mut active: login::ActiveModel = row.into();
    active.auth_enabled = Set(Some(1));
    active.update(db).await?;
    Ok(true)
}

// ── Sub-users (brand-restricted logins) ─────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SubUserRow {
    pub id: i32,
    pub username: String,
    pub name: Option<String>,
    pub auth_enabled: bool,
}

/// List sub-users for a brand. Filters `login` by `app = brand_id` and
/// `tied_to IS NOT NULL` so the main user is never returned.
pub async fn list_sub_users(
    db: &DatabaseConnection,
    brand_id: i32,
) -> AppResult<Vec<SubUserRow>> {
    let rows = login::Entity::find()
        .filter(login::Column::App.eq(brand_id))
        .filter(login::Column::TiedTo.is_not_null())
        .order_by_asc(login::Column::Id)
        .all(db)
        .await?;
    Ok(rows
        .into_iter()
        .map(|r| SubUserRow {
            id: r.id,
            username: r.username.unwrap_or_default(),
            name: r.name,
            auth_enabled: r.auth_enabled.unwrap_or(0) == 1,
        })
        .collect())
}

/// Create a brand-restricted login. The new row's `tied_to` points at the
/// admin who created it; `app` is the brand id; password is argon2id.
pub async fn create_sub_user(
    db: &DatabaseConnection,
    main_user_id: i32,
    brand_id: i32,
    email: &str,
    password: &str,
    name: Option<String>,
) -> AppResult<i32> {
    let email = email.trim();
    if email.is_empty() {
        return Err(AppError::BadRequest("email required".into()));
    }
    if password.len() < 8 {
        return Err(AppError::BadRequest("password must be at least 8 chars".into()));
    }
    if login::Entity::find()
        .filter(login::Column::Username.eq(email))
        .one(db)
        .await?
        .is_some()
    {
        return Err(AppError::BadRequest("a user with that email already exists".into()));
    }

    let hashed = hash_password(password)?;
    let active = login::ActiveModel {
        username: Set(Some(email.to_string())),
        password: Set(Some(hashed)),
        name: Set(name),
        tied_to: Set(Some(main_user_id)),
        app: Set(Some(brand_id)),
        cron: Set(Some(0)),
        auth_enabled: Set(Some(0)),
        ..Default::default()
    };
    Ok(active.insert(db).await?.id)
}

pub async fn delete_sub_user(db: &DatabaseConnection, user_id: i32) -> AppResult<()> {
    // Refuse to delete the main user (tied_to NULL) — even via this path.
    let row = login::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("user {user_id}")))?;
    if row.tied_to.is_none() {
        return Err(AppError::BadRequest("cannot delete the main admin user".into()));
    }
    login::Entity::delete_by_id(user_id).exec(db).await?;
    Ok(())
}

/// Disable 2FA for a user (clears `auth_enabled`).
pub async fn disable_totp(db: &DatabaseConnection, user_id: i32) -> AppResult<()> {
    let row = login::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("user {user_id}")))?;
    let mut active: login::ActiveModel = row.into();
    active.auth_enabled = Set(Some(0));
    active.auth_key = Set(Some(String::new()));
    active.update(db).await?;
    Ok(())
}

// ── Password reset ───────────────────────────────────────────────────────────

/// Request a password reset for `username`. Generates a random reset token,
/// stores it on the **main user** row (id = lowest), and returns the token
/// + the target user-id so the caller can email a reset link.
///
/// Returns `Ok(None)` when no user with that username exists (the caller
/// should still respond positively to avoid leaking existence).
pub async fn request_password_reset(
    db: &DatabaseConnection,
    username: &str,
) -> AppResult<Option<(String, i32)>> {
    let user = login::Entity::find()
        .filter(login::Column::Username.eq(username))
        .one(db)
        .await?;
    let Some(user) = user else { return Ok(None) };

    let main = login::Entity::find()
        .order_by_asc(login::Column::Id)
        .one(db)
        .await?;
    let Some(main) = main else { return Ok(None) };

    let token = random_token(40);
    let mut active: login::ActiveModel = main.into();
    active.reset_password_key = Set(Some(token.clone()));
    active.update(db).await?;

    Ok(Some((token, user.id)))
}

/// Complete a password-reset flow: rotate the user's password to
/// `new_password`, clear the main user's reset key. Returns `Ok(false)` when
/// the supplied `token` does not match the stored reset key.
pub async fn reset_password(
    db: &DatabaseConnection,
    token: &str,
    user_id: i32,
    new_password: &str,
) -> AppResult<bool> {
    let main = login::Entity::find()
        .order_by_asc(login::Column::Id)
        .one(db)
        .await?;
    let Some(main) = main else { return Ok(false) };

    let stored = main.reset_password_key.clone().unwrap_or_default();
    if stored.is_empty() || stored != token {
        return Ok(false);
    }

    let user = login::Entity::find_by_id(user_id).one(db).await?;
    let Some(user) = user else { return Ok(false) };

    let hashed = hash_password(new_password)?;
    let mut active: login::ActiveModel = user.into();
    active.password = Set(Some(hashed));
    active.update(db).await?;

    let mut main_active: login::ActiveModel = main.into();
    main_active.reset_password_key = Set(Some(String::new()));
    main_active.update(db).await?;

    Ok(true)
}

fn random_token(len: usize) -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}

