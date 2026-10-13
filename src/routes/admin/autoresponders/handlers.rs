use super::views::*;
use leptos::prelude::*;
use poem::{
    handler,
    http::StatusCode,
    web::{Data, Form, Path, Query},
    Response,
};
use serde::Deserialize;
use crate::routes::admin::session::CurrentUser;
use crate::services::autoresponders::{self, AutoresponderEmailInput, AutoresponderInput};
use crate::utils::opaque_id::{Brand, OpaqueId};
use crate::AppState;

/// `GET /app/lists/:id/autoresponders`
#[handler]
pub async fn list_for_list_page(
    _user: CurrentUser,
    Path(list_token): Path<crate::utils::opaque_id::OpaqueId<crate::utils::opaque_id::List>>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let list = match crate::services::lists::find_list(&state.db, list_token.raw()).await {
        Ok(Some(l)) => l,
        _ => return redirect("/app/brands"),
    };
    let rows = autoresponders::list_for_list(&state.db, list_token.raw())
        .await
        .unwrap_or_default();
    let list_name = list.name.unwrap_or_default();
    let token = list_token.to_string();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Autoresponders", move || {
        view! { <ListAutorespondersPage list_token=token list_name=list_name rows=rows/> }
    })
    .await
}

/// `POST /app/lists/:id/autoresponders/new`
#[handler]
pub async fn list_create_ar(
    _user: CurrentUser,
    Path(list_token): Path<crate::utils::opaque_id::OpaqueId<crate::utils::opaque_id::List>>,
    Form(form): Form<ListAresForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = autoresponders::create(
        &state.db,
        AutoresponderInput {
            name: form.name.trim().to_string(),
            list_id: list_token.raw(),
            kind: 0,
            custom_field: None,
        },
    )
    .await;
    redirect(&format!("/app/lists/{list_token}/autoresponders"))
}

#[derive(Debug, Deserialize)]
pub struct BrandQuery {
    pub brand: String,
}

#[handler]
pub async fn list_for_brand_page(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let brand = match crate::services::brands::find_brand(&state.db, brand_token.raw()).await {
        Ok(Some(b)) => b,
        _ => return redirect("/app/brands"),
    };
    let rows = autoresponders::list_for_brand(&state.db, brand_token.raw())
        .await
        .unwrap_or_default();
    let token = brand_token.to_string();
    let brand_name = brand.app_name.unwrap_or_default();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Autoresponders", move || {
        view! { <AutorespondersListPage brand_token=token brand_name=brand_name rows=rows/> }
    })
    .await
}

#[handler]
pub async fn create(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Form(form): Form<AresForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = autoresponders::create(
        &state.db,
        AutoresponderInput {
            name: form.name.trim().to_string(),
            list_id: form.list_id,
            kind: 0,
            custom_field: None,
        },
    )
    .await;
    redirect(&format!("/app/brands/{brand_token}/autoresponders"))
}

#[handler]
pub async fn delete(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Query(q): Query<BrandQuery>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = autoresponders::delete(&state.db, id).await;
    redirect(&format!("/app/brands/{}/autoresponders", q.brand))
}

#[handler]
pub async fn emails_page(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let row = match autoresponders::find(&state.db, id).await {
        Ok(Some(a)) => a,
        _ => return redirect("/app/brands"),
    };
    let emails = autoresponders::list_emails(&state.db, id).await.unwrap_or_default();
    let name = row.name.unwrap_or_default();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Autoresponder emails", move || {
        view! { <EmailsPage autoresponder_id=id autoresponder_name=name emails=emails/> }
    })
    .await
}

// ── Autoresponder email handlers ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct EmailForm {
    pub title: String,
    pub from_name: Option<String>,
    pub from_email: Option<String>,
    pub reply_to: Option<String>,
    pub time_condition: Option<String>,
    pub timezone: Option<String>,
    pub html_text: Option<String>,
    pub plain_text: Option<String>,
    pub opens_tracking: Option<String>,
    pub links_tracking: Option<String>,
    /// Repeated `segs` checkbox values from the include list.
    #[serde(default)]
    pub segs: Vec<i32>,
    /// Repeated `segs_excl` checkbox values from the exclude list.
    #[serde(default)]
    pub segs_excl: Vec<i32>,
}

impl EmailForm {
    fn into_input(self, ares_id: i32) -> AutoresponderEmailInput {
        AutoresponderEmailInput {
            ares_id,
            title: self.title.trim().to_string(),
            from_name: nonempty(self.from_name),
            from_email: nonempty(self.from_email),
            reply_to: nonempty(self.reply_to),
            plain_text: nonempty(self.plain_text),
            html_text: nonempty(self.html_text),
            time_condition: nonempty(self.time_condition),
            timezone: nonempty(self.timezone),
            opens_tracking: self.opens_tracking.is_some(),
            links_tracking: self.links_tracking.is_some(),
            segs: csv_or_none(&self.segs),
            segs_excl: csv_or_none(&self.segs_excl),
        }
    }
}

fn csv_or_none(ids: &[i32]) -> Option<String> {
    if ids.is_empty() {
        None
    } else {
        Some(
            ids.iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(","),
        )
    }
}

fn nonempty(s: Option<String>) -> Option<String> {
    s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// `GET /app/autoresponders/:id/emails/new`
#[handler]
pub async fn email_new_form(
    _user: CurrentUser,
    Path(ares_id): Path<i32>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let row = match autoresponders::find(&state.db, ares_id).await {
        Ok(Some(a)) => a,
        _ => return redirect("/app/brands"),
    };
    let name = row.name.unwrap_or_default();
    let segments = load_segment_options(&state.db, row.list.unwrap_or_default()).await;
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "New email", move || {
        view! { <EmailFormPage autoresponder_id=ares_id autoresponder_name=name segments=segments/> }
    })
    .await
}

/// Look up the segments under a list and shape them for the EmailFormPage
/// segments multi-select.
async fn load_segment_options(db: &sea_orm::DatabaseConnection, list_id: i32) -> Vec<SegmentOption> {
    if list_id == 0 {
        return Vec::new();
    }
    crate::services::segments::list_for_list(db, list_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|s| SegmentOption {
            id: s.id,
            name: s.name.unwrap_or_else(|| format!("Segment #{}", s.id)),
        })
        .collect()
}

/// `POST /app/autoresponders/:id/emails/new` — create.
#[handler]
pub async fn email_create(
    _user: CurrentUser,
    Path(ares_id): Path<i32>,
    Form(form): Form<EmailForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = autoresponders::create_email(&state.db, form.into_input(ares_id)).await;
    redirect(&format!("/app/autoresponders/{ares_id}/emails"))
}

/// `GET /app/autoresponder-emails/:id/edit`
#[handler]
pub async fn email_edit_form(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let email = match autoresponders::find_email(&state.db, id).await {
        Ok(Some(e)) => e,
        _ => return redirect("/app/brands"),
    };
    let ares_id = email.ares_id.unwrap_or_default();
    let ares_name = autoresponders::find(&state.db, ares_id)
        .await
        .ok()
        .flatten()
        .and_then(|a| a.name)
        .unwrap_or_default();
    let attachments = crate::services::attachments::list_for_ares(id).await;
    let list_id = autoresponders::find(&state.db, ares_id)
        .await
        .ok()
        .flatten()
        .and_then(|a| a.list)
        .unwrap_or_default();
    let segments = load_segment_options(&state.db, list_id).await;
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Edit email", move || {
        view! { <EmailFormPage autoresponder_id=ares_id autoresponder_name=ares_name existing=email attachments=attachments segments=segments/> }
    })
    .await
}

/// `POST /app/autoresponder-emails/:id/edit` — update.
#[handler]
pub async fn email_update(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Form(form): Form<EmailForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let ares_id = autoresponders::find_email(&state.db, id)
        .await
        .ok()
        .flatten()
        .and_then(|e| e.ares_id)
        .unwrap_or(0);
    let _ = autoresponders::update_email(&state.db, id, form.into_input(ares_id)).await;
    redirect(&format!("/app/autoresponders/{ares_id}/emails"))
}

/// `POST /app/autoresponder-emails/:id/toggle` — flip enabled flag.
#[handler]
pub async fn email_toggle(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Data(state): Data<&AppState>,
) -> Response {
    let ares_id = autoresponders::find_email(&state.db, id)
        .await
        .ok()
        .flatten()
        .and_then(|e| e.ares_id)
        .unwrap_or(0);
    let _ = autoresponders::toggle_email(&state.db, id).await;
    redirect(&format!("/app/autoresponders/{ares_id}/emails"))
}

/// `POST /app/autoresponder-emails/:id/delete`
#[handler]
pub async fn email_delete(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Data(state): Data<&AppState>,
) -> Response {
    let ares_id = autoresponders::find_email(&state.db, id)
        .await
        .ok()
        .flatten()
        .and_then(|e| e.ares_id)
        .unwrap_or(0);
    let _ = autoresponders::delete_email(&state.db, id).await;
    redirect(&format!("/app/autoresponders/{ares_id}/emails"))
}

/// `POST /app/autoresponder-emails/:id/attachments/upload` — multipart
/// upload of one attachment file to an AR email.
#[handler]
pub async fn email_upload_attachment(
    _user: CurrentUser,
    Path(id): Path<i32>,
    mut multipart: poem::web::Multipart,
    Data(_state): Data<&AppState>,
) -> Response {
    let mut filename: Option<String> = None;
    let mut bytes: Vec<u8> = Vec::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            filename = field.file_name().map(|s| s.to_string());
            if let Ok(b) = field.bytes().await {
                bytes = b;
            }
            break;
        }
    }
    let filename = match filename.filter(|s| !s.is_empty()) {
        Some(n) => n,
        None => {
            return redirect(&format!(
                "/app/autoresponder-emails/{id}/edit?e=attach_missing"
            ))
        }
    };
    if bytes.is_empty() {
        return redirect(&format!(
            "/app/autoresponder-emails/{id}/edit?e=attach_empty"
        ));
    }
    if bytes.len() > crate::services::attachments::MAX_ATTACHMENT_BYTES {
        return redirect(&format!(
            "/app/autoresponder-emails/{id}/edit?e=attach_too_large"
        ));
    }
    if let Err(e) = crate::services::attachments::save_for_ares(id, &filename, &bytes).await {
        tracing::warn!(error = ?e, "ares attachment save failed");
        return redirect(&format!(
            "/app/autoresponder-emails/{id}/edit?e=attach_save"
        ));
    }
    redirect(&format!("/app/autoresponder-emails/{id}/edit?i=attached"))
}

#[derive(Debug, Deserialize)]
pub struct DeleteAresAttachmentForm {
    pub filename: String,
}

/// `POST /app/autoresponder-emails/:id/attachments/delete`
#[handler]
pub async fn email_delete_attachment(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Form(form): Form<DeleteAresAttachmentForm>,
    Data(_state): Data<&AppState>,
) -> Response {
    let _ = crate::services::attachments::delete_for_ares(id, &form.filename).await;
    redirect(&format!(
        "/app/autoresponder-emails/{id}/edit?i=attach_deleted"
    ))
}

/// `GET /app/autoresponder-emails/:id/report`
#[handler]
pub async fn email_report(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let report = crate::services::reports::build_ares_email(&state.db, id)
        .await
        .unwrap_or_default();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "AR email report", move || {
        view! { <EmailReportPage report=report/> }
    })
    .await
}

/// `GET /app/autoresponder-emails/:id/report.csv`
#[handler]
pub async fn email_report_csv(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Data(state): Data<&AppState>,
) -> Response {
    let report = crate::services::reports::build_ares_email(&state.db, id)
        .await
        .unwrap_or_default();
    let mut wtr = csv::Writer::from_writer(Vec::new());
    let _ = wtr.write_record(&["metric", "value"]);
    let _ = wtr.write_record(&["email_id", &report.email_id.to_string()]);
    let _ = wtr.write_record(&["title", &report.title]);
    let _ = wtr.write_record(&["recipients", &report.recipients.to_string()]);
    let _ = wtr.write_record(&["unique_opens", &report.unique_opens.to_string()]);
    let _ = wtr.write_record(&["total_opens", &report.total_opens.to_string()]);
    let _ = wtr.write_record(&["unique_clickers", &report.unique_clickers.to_string()]);
    let _ = wtr.write_record(&["total_clicks", &report.total_clicks.to_string()]);
    for l in &report.top_links {
        let _ = wtr.write_record(&[format!("link:{}", l.url), l.clicks.to_string()]);
    }
    let body = wtr.into_inner().unwrap_or_default();
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/csv; charset=utf-8")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"ares-email-{id}-report.csv\""),
        )
        .body(body)
}

/// `POST /app/autoresponder-emails/:id/test-send` — render with a
/// dummy subscriber and ship one preview email. Mirrors the campaign
/// `test_send` handler.
#[handler]
pub async fn email_test_send(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Form(form): Form<TestSendForm>,
    Data(state): Data<&AppState>,
) -> Response {
    use crate::database::entities::{apps, lists, subscribers as subs_entity};
    use crate::services::mailer::{mailer_for_brand, OutboundEmail, StubMailer};
    use crate::services::personalization::{render, Context};
    use sea_orm::EntityTrait;

    let to_email = form.email.trim().to_string();
    if to_email.is_empty() {
        return redirect(&format!(
            "/app/autoresponder-emails/{id}/edit?e=test_email"
        ));
    }
    let email = match autoresponders::find_email(&state.db, id).await {
        Ok(Some(e)) => e,
        _ => return redirect("/app/brands"),
    };
    let ares_id = email.ares_id.unwrap_or_default();
    // Resolve the parent autoresponder → list → brand chain.
    let ares = autoresponders::find(&state.db, ares_id).await.ok().flatten();
    let list_id = ares.as_ref().and_then(|a| a.list).unwrap_or_default();
    let brand = match lists::Entity::find_by_id(list_id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .and_then(|l| l.app)
    {
        Some(app_id) => apps::Entity::find_by_id(app_id)
            .one(&state.db)
            .await
            .ok()
            .flatten(),
        None => None,
    };

    let dummy = subs_entity::Model {
        id: 0,
        user_id: None,
        name: Some("Test Recipient".into()),
        email: Some(to_email.clone()),
        list: Some(list_id),
        custom_fields: Some(String::new()),
        unsubscribed: Some(0),
        bounced: Some(0),
        bounce_soft: Some(0),
        complaint: Some(0),
        confirmed: Some(1),
        last_campaign: None,
        last_ares: None,
        timestamp: Some(chrono::Utc::now().timestamp()),
        join_date: Some(chrono::Utc::now().timestamp()),
        message_id: None,
        ip: None,
        country: None,
        referrer: None,
        method: Some(1),
        added_via: Some(1),
        notes: None,
        gdpr: Some(0),
    };
    let app_url = state.config.app_url.clone();
    let ctx = Context::build(&dummy, list_id, 0, "", &app_url);

    let from_name = email
        .from_name
        .clone()
        .or_else(|| brand.as_ref().and_then(|b| b.from_name.clone()))
        .unwrap_or_default();
    let from_email = email
        .from_email
        .clone()
        .or_else(|| brand.as_ref().and_then(|b| b.from_email.clone()))
        .unwrap_or_default();
    let reply_to = email
        .reply_to
        .clone()
        .or_else(|| brand.as_ref().and_then(|b| b.reply_to.clone()))
        .unwrap_or_else(|| from_email.clone());
    let subject = render(
        &format!("[TEST] {}", email.title.clone().unwrap_or_default()),
        &ctx,
    );
    let html = render(&email.html_text.clone().unwrap_or_default(), &ctx);
    let plain = render(&email.plain_text.clone().unwrap_or_default(), &ctx);

    let mailer: Box<dyn crate::services::mailer::EmailProvider> = brand
        .as_ref()
        .map(mailer_for_brand)
        .unwrap_or_else(|| Box::new(StubMailer));
    let attachments = crate::services::attachments::load_for_ares_email(id).await;
    let outbound = OutboundEmail {
        from_name,
        from_email,
        to_email,
        to_name: Some("Test".into()),
        reply_to,
        subject,
        html,
        plain,
        attachments,
    };
    if let Err(e) = mailer.send(outbound).await {
        tracing::warn!(error = ?e, "ares email test send failed");
        return redirect(&format!(
            "/app/autoresponder-emails/{id}/edit?e=test_send"
        ));
    }
    redirect(&format!(
        "/app/autoresponder-emails/{id}/edit?i=test_sent"
    ))
}

#[derive(Debug, Deserialize)]
pub struct TestSendForm {
    pub email: String,
}

#[handler]
pub async fn email_duplicate(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Data(state): Data<&AppState>,
) -> Response {
    let ares_id = autoresponders::find_email(&state.db, id)
        .await
        .ok()
        .flatten()
        .and_then(|e| e.ares_id)
        .unwrap_or(0);
    let _ = autoresponders::duplicate_email(&state.db, id).await;
    redirect(&format!("/app/autoresponders/{ares_id}/emails"))
}

/// `GET /app/autoresponder-emails/:id/preview` — render the AR-email body
/// with a dummy subscriber context and return a stand-alone HTML preview.
/// `?view=plain` opens the plain-text pane on first paint.
#[handler]
pub async fn email_preview(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Query(q): Query<PreviewQuery>,
    Data(state): Data<&AppState>,
) -> Response {
    use crate::services::personalization::Context;
    use crate::services::preview::{build_preview, dummy_subscriber, PreviewMeta, PreviewMode};

    let row = match autoresponders::find_email(&state.db, id).await {
        Ok(Some(e)) => e,
        _ => return redirect("/app/brands"),
    };

    // Resolve the parent autoresponder so we can derive a list_id for
    // [unsubscribe] / [preferences] URLs in the preview.
    let list_id = match row.ares_id {
        Some(aid) => autoresponders::find(&state.db, aid)
            .await
            .ok()
            .flatten()
            .and_then(|a| a.list)
            .unwrap_or(0),
        None => 0,
    };

    let dummy = dummy_subscriber("preview@mailer.local", "Preview Recipient");
    let app_url = state.config.app_url.clone();
    let ctx = Context::build(&dummy, list_id, 0, "", &app_url);

    let from_name = row.from_name.clone().unwrap_or_default();
    let from_email = row.from_email.clone().unwrap_or_default();
    let subject = row.title.clone().unwrap_or_default();
    let html_body = row.html_text.clone().unwrap_or_default();
    let plain_body = row.plain_text.clone().unwrap_or_default();
    let back_href = format!("/app/autoresponder-emails/{id}/edit");
    let mode = PreviewMode::from_query(q.view.as_deref());

    let html = build_preview(
        &html_body,
        &plain_body,
        mode,
        PreviewMeta {
            kind: "Autoresponder email",
            subject: &subject,
            from_name: &from_name,
            from_email: &from_email,
            back_href: &back_href,
        },
        &ctx,
    );
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("X-Robots-Tag", "none")
        .body(html)
}

#[derive(Debug, Deserialize)]
pub struct PreviewQuery {
    pub view: Option<String>,
}

fn redirect(location: &str) -> Response {
    Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", location)
        .body(())
}
