use super::views::*;
use leptos::prelude::*;
use poem::{
    handler,
    http::StatusCode,
    web::{Data, Form, Path, Query},
    Response,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};
use serde::Deserialize;
use crate::database::entities::campaigns;
use crate::routes::admin::session::CurrentUser;
use crate::utils::opaque_id::{Brand, Campaign, OpaqueId};
use crate::AppState;

#[handler]
pub async fn list_for_brand_page(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Query(q): Query<PageQuery>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let brand = match crate::services::brands::find_brand(&state.db, brand_token.raw()).await {
        Ok(Some(b)) => b,
        _ => return redirect("/app/brands"),
    };
    let page = q.page.unwrap_or(0);
    let paginator = campaigns::Entity::find()
        .filter(campaigns::Column::App.eq(brand_token.raw()))
        .order_by_desc(campaigns::Column::Id)
        .paginate(&state.db, PAGE_SIZE);
    let total = paginator.num_items().await.unwrap_or(0);
    let rows = paginator.fetch_page(page).await.unwrap_or_default();
    let token = brand_token.to_string();
    let brand_name = brand.app_name.unwrap_or_default();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Campaigns", move || {
        view! { <CampaignsListPage brand_token=token brand_name=brand_name rows=rows page=page total=total/> }
    })
    .await
}

#[handler]
pub async fn new_form(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let token = brand_token.to_string();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "New campaign", move || {
        view! { <CampaignFormPage brand_token=token/> }
    })
    .await
}

#[handler]
pub async fn edit_form(
    _user: CurrentUser,
    Path(campaign_token): Path<OpaqueId<Campaign>>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let row = match campaigns::Entity::find_by_id(campaign_token.raw()).one(&state.db).await {
        Ok(Some(c)) => c,
        _ => return redirect("/app/brands"),
    };
    let brand_id = row.app.unwrap_or_default();
    let brand_token = OpaqueId::<Brand>::new(brand_id).to_string();
    let attachments = crate::services::attachments::list(campaign_token.raw()).await;
    let broken_links = crate::services::campaigns::check_broken_links(
        row.html_text.as_deref().unwrap_or(""),
    );
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Edit campaign", move || {
        view! { <CampaignFormPage brand_token=brand_token existing=row attachments=attachments broken_links=broken_links/> }
    })
    .await
}

#[handler]
pub async fn create(
    user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Form(form): Form<CampaignForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let save_template = form.save_as_template.is_some();
    let html_text = nonempty(form.html_text.clone());
    let plain_text = nonempty(form.plain_text.clone());
    let title = form.title.clone();
    let active = campaigns::ActiveModel {
        user_id: Set(Some(user.0.id)),
        app: Set(Some(brand_token.raw())),
        from_name: Set(nonempty(form.from_name.clone())),
        from_email: Set(nonempty(form.from_email.clone())),
        reply_to: Set(nonempty(form.reply_to.clone())),
        title: Set(Some(form.title.clone())),
        subject: Set(Some(form.subject.clone())),
        html_text: Set(html_text.clone()),
        plain_text: Set(plain_text.clone()),
        query_string: Set(nonempty(form.query_string.clone())),
        wysiwyg: Set(Some(1)),
        recipients: Set(Some(0)),
        to_send: Set(Some(0)),
        opens_tracking: Set(Some(1)),
        links_tracking: Set(Some(1)),
        ..Default::default()
    };
    match active.insert(&state.db).await {
        Ok(_) => {
            if save_template {
                let _ = crate::services::templates::create(
                    &state.db,
                    user.0.id,
                    brand_token.raw(),
                    crate::services::templates::TemplateInput {
                        template_name: title,
                        from_name: nonempty(form.from_name),
                        from_email: nonempty(form.from_email),
                        reply_to: nonempty(form.reply_to),
                        html_text,
                        plain_text,
                    },
                )
                .await;
            }
            redirect(&format!("/app/brands/{}/campaigns", brand_token))
        }
        Err(e) => {
            tracing::error!(error = ?e, "create campaign failed");
            redirect(&format!("/app/brands/{}/campaigns/new", brand_token))
        }
    }
}

#[handler]
pub async fn update(
    user: CurrentUser,
    Path(campaign_token): Path<OpaqueId<Campaign>>,
    Form(form): Form<CampaignForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let row = match campaigns::Entity::find_by_id(campaign_token.raw()).one(&state.db).await {
        Ok(Some(c)) => c,
        _ => return redirect("/app/brands"),
    };
    let brand_id = row.app.unwrap_or_default();
    let save_template = form.save_as_template.is_some();
    let html_for_template = nonempty(form.html_text.clone());
    let plain_for_template = nonempty(form.plain_text.clone());
    let title_for_template = form.title.clone();
    let from_for_template = (
        nonempty(form.from_name.clone()),
        nonempty(form.from_email.clone()),
        nonempty(form.reply_to.clone()),
    );
    let mut active: campaigns::ActiveModel = row.into();
    active.title = Set(Some(form.title));
    active.subject = Set(Some(form.subject));
    active.from_name = Set(nonempty(form.from_name));
    active.from_email = Set(nonempty(form.from_email));
    active.reply_to = Set(nonempty(form.reply_to));
    active.html_text = Set(nonempty(form.html_text));
    active.plain_text = Set(nonempty(form.plain_text));
    active.query_string = Set(nonempty(form.query_string));
    let _ = active.update(&state.db).await;
    if save_template {
        let _ = crate::services::templates::create(
            &state.db,
            user.0.id,
            brand_id,
            crate::services::templates::TemplateInput {
                template_name: title_for_template,
                from_name: from_for_template.0,
                from_email: from_for_template.1,
                reply_to: from_for_template.2,
                html_text: html_for_template,
                plain_text: plain_for_template,
            },
        )
        .await;
    }
    redirect(&format!("/app/campaigns/{}/edit", campaign_token))
}

#[derive(Debug, Deserialize)]
pub struct ScheduleForm {
    pub send_at: String,
    pub timezone: String,
    pub list_ids: String,
}

#[derive(Debug, Deserialize)]
pub struct ImportUrlForm {
    pub url: String,
}

/// `POST /app/campaigns/:id/import-url` — fetch the URL and stash the
/// response body as the campaign's html_text.
#[handler]
pub async fn import_url(
    _user: CurrentUser,
    Path(campaign_token): Path<OpaqueId<Campaign>>,
    Form(form): Form<ImportUrlForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let url = form.url.trim().to_string();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return redirect(&format!("/app/campaigns/{}/edit?e=bad_url", campaign_token));
    }

    let body = match reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
    {
        Ok(resp) => match resp.text().await {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(error = ?e, %url, "import-url body read failed");
                return redirect(&format!("/app/campaigns/{}/edit?e=fetch_failed", campaign_token));
            }
        },
        Err(e) => {
            tracing::warn!(error = ?e, %url, "import-url HTTP failed");
            return redirect(&format!("/app/campaigns/{}/edit?e=fetch_failed", campaign_token));
        }
    };

    if let Ok(Some(c)) = campaigns::Entity::find_by_id(campaign_token.raw())
        .one(&state.db)
        .await
    {
        let mut active: campaigns::ActiveModel = c.into();
        active.html_text = Set(Some(body));
        let _ = active.update(&state.db).await;
    }
    redirect(&format!("/app/campaigns/{}/edit?i=imported", campaign_token))
}

/// `POST /app/campaigns/:id/import-html` — multipart upload of an HTML file.
#[handler]
pub async fn import_html(
    _user: CurrentUser,
    Path(campaign_token): Path<OpaqueId<Campaign>>,
    mut multipart: poem::web::Multipart,
    Data(state): Data<&AppState>,
) -> Response {
    let mut bytes: Vec<u8> = Vec::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("html") {
            if let Ok(b) = field.bytes().await {
                bytes = b;
                break;
            }
        }
    }
    if bytes.is_empty() {
        return redirect(&format!("/app/campaigns/{}/edit?e=empty_html", campaign_token));
    }
    if bytes.len() > 5 * 1024 * 1024 {
        return redirect(&format!("/app/campaigns/{}/edit?e=html_too_large", campaign_token));
    }
    let body = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => return redirect(&format!("/app/campaigns/{}/edit?e=bad_encoding", campaign_token)),
    };

    if let Ok(Some(c)) = campaigns::Entity::find_by_id(campaign_token.raw())
        .one(&state.db)
        .await
    {
        let mut active: campaigns::ActiveModel = c.into();
        active.html_text = Set(Some(body));
        let _ = active.update(&state.db).await;
    }
    redirect(&format!("/app/campaigns/{}/edit?i=imported", campaign_token))
}

/// `POST /app/campaigns/:id/dismiss-broken-link-checks` — flip
/// `campaigns.ignore_checks = 1`.
#[handler]
pub async fn dismiss_link_check(
    _user: CurrentUser,
    Path(campaign_token): Path<OpaqueId<Campaign>>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = crate::services::campaigns::dismiss_broken_link_check(
        &state.db,
        campaign_token.raw(),
    )
    .await;
    redirect(&format!(
        "/app/campaigns/{}/edit?i=link_check_dismissed",
        campaign_token
    ))
}

/// `POST /app/campaigns/:id/attachments/upload` — multipart upload of one
/// attachment file. Honors the brand's `allowed_attachments` extension list.
#[handler]
pub async fn upload_attachment(
    _user: CurrentUser,
    Path(campaign_token): Path<OpaqueId<Campaign>>,
    mut multipart: poem::web::Multipart,
    Data(state): Data<&AppState>,
) -> Response {
    let campaign_id = campaign_token.raw();

    let campaign = match campaigns::Entity::find_by_id(campaign_id).one(&state.db).await {
        Ok(Some(c)) => c,
        _ => return redirect("/app/brands"),
    };
    let allowed = match campaign.app {
        Some(app_id) => match crate::database::entities::apps::Entity::find_by_id(app_id)
            .one(&state.db)
            .await
        {
            Ok(Some(b)) => b.allowed_attachments,
            _ => None,
        },
        None => None,
    };

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
                "/app/campaigns/{}/edit?e=attach_missing",
                campaign_token
            ))
        }
    };
    if bytes.is_empty() {
        return redirect(&format!(
            "/app/campaigns/{}/edit?e=attach_empty",
            campaign_token
        ));
    }
    if bytes.len() > crate::services::attachments::MAX_ATTACHMENT_BYTES {
        return redirect(&format!(
            "/app/campaigns/{}/edit?e=attach_too_large",
            campaign_token
        ));
    }
    if !crate::services::attachments::extension_allowed(allowed.as_deref(), &filename) {
        return redirect(&format!(
            "/app/campaigns/{}/edit?e=attach_extension",
            campaign_token
        ));
    }

    if let Err(e) = crate::services::attachments::save(campaign_id, &filename, &bytes).await {
        tracing::warn!(error = ?e, "attachment save failed");
        return redirect(&format!(
            "/app/campaigns/{}/edit?e=attach_save",
            campaign_token
        ));
    }
    redirect(&format!(
        "/app/campaigns/{}/edit?i=attached",
        campaign_token
    ))
}

#[derive(Debug, Deserialize)]
pub struct DeleteAttachmentForm {
    pub filename: String,
}

/// `POST /app/campaigns/:id/attachments/delete` — remove one attachment.
#[handler]
pub async fn delete_attachment(
    _user: CurrentUser,
    Path(campaign_token): Path<OpaqueId<Campaign>>,
    Form(form): Form<DeleteAttachmentForm>,
    Data(_state): Data<&AppState>,
) -> Response {
    let _ = crate::services::attachments::delete(campaign_token.raw(), &form.filename).await;
    redirect(&format!(
        "/app/campaigns/{}/edit?i=attach_deleted",
        campaign_token
    ))
}

/// `POST /app/campaigns/:id/schedule` — set send_date + timezone + lists.
/// The `send_at` field arrives as a `<input type="datetime-local">` in the
/// user's selected IANA timezone; we interpret it in that zone and store
/// the resulting UTC unix-timestamp so the scheduler can compare directly
/// to `chrono::Utc::now().timestamp()`.
#[handler]
pub async fn schedule(
    _user: CurrentUser,
    Path(campaign_token): Path<OpaqueId<Campaign>>,
    Form(form): Form<ScheduleForm>,
    Data(state): Data<&AppState>,
) -> Response {
    use chrono::TimeZone;

    let tz: chrono_tz::Tz = form
        .timezone
        .trim()
        .parse()
        .unwrap_or(chrono_tz::UTC);

    let naive = match chrono::NaiveDateTime::parse_from_str(form.send_at.trim(), "%Y-%m-%dT%H:%M")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(form.send_at.trim(), "%Y-%m-%dT%H:%M:%S"))
    {
        Ok(n) => n,
        Err(_) => return redirect(&format!("/app/campaigns/{}/edit?e=bad_date", campaign_token)),
    };
    let local = match tz.from_local_datetime(&naive).single() {
        Some(d) => d,
        None => return redirect(&format!("/app/campaigns/{}/edit?e=ambiguous_date", campaign_token)),
    };
    let send_at = local.with_timezone(&chrono::Utc).timestamp();

    let row = match campaigns::Entity::find_by_id(campaign_token.raw())
        .one(&state.db)
        .await
    {
        Ok(Some(c)) => c,
        _ => return redirect("/app/brands"),
    };
    let mut active: campaigns::ActiveModel = row.into();
    active.send_date = Set(Some(send_at.to_string()));
    active.timezone = Set(Some(tz.name().to_string()));
    active.lists = Set(Some(form.list_ids.clone()));
    active.to_send_lists = Set(Some(form.list_ids));
    let _ = active.update(&state.db).await;
    redirect(&format!("/app/campaigns/{}/edit?i=scheduled", campaign_token))
}

#[derive(Debug, Deserialize)]
pub struct TestSendForm {
    pub email: String,
}

/// `POST /app/campaigns/:id/test-send` — render the campaign with a dummy
/// subscriber context and send to a single recipient.
#[handler]
pub async fn test_send(
    _user: CurrentUser,
    Path(campaign_token): Path<OpaqueId<Campaign>>,
    Form(form): Form<TestSendForm>,
    Data(state): Data<&AppState>,
) -> Response {
    use crate::database::entities::{apps, subscribers as subs_entity};
    use crate::services::mailer::{mailer_for_brand, OutboundEmail, StubMailer};
    use crate::services::personalization::{render, Context};

    let to_email = form.email.trim().to_string();
    if to_email.is_empty() {
        return redirect(&format!("/app/campaigns/{}/edit?e=test_email", campaign_token));
    }

    let campaign = match campaigns::Entity::find_by_id(campaign_token.raw())
        .one(&state.db)
        .await
    {
        Ok(Some(c)) => c,
        _ => return redirect("/app/brands"),
    };

    let brand = match campaign.app {
        Some(id) => apps::Entity::find_by_id(id).one(&state.db).await.ok().flatten(),
        None => None,
    };

    // Synthetic subscriber so personalization tags don't blow up.
    let dummy = subs_entity::Model {
        id: 0, user_id: None, name: Some("Test Recipient".into()),
        email: Some(to_email.clone()), list: Some(0), custom_fields: Some(String::new()),
        unsubscribed: Some(0), bounced: Some(0), bounce_soft: Some(0), complaint: Some(0),
        confirmed: Some(1), last_campaign: None, last_ares: None,
        timestamp: Some(chrono::Utc::now().timestamp()),
        join_date: Some(chrono::Utc::now().timestamp()),
        message_id: None, ip: None, country: None, referrer: None,
        method: Some(1), added_via: Some(1), notes: None, gdpr: Some(0),
    };

    let app_url = state.config.app_url.clone();
    let ctx = Context::build(&dummy, 0, campaign.id, "", &app_url);

    let from_name = campaign.from_name.clone().unwrap_or_default();
    let from_email = campaign.from_email.clone().unwrap_or_default();
    let reply_to = campaign.reply_to.clone().unwrap_or(from_email.clone());
    let subject = render(
        &format!("[TEST] {}", campaign.subject.clone().unwrap_or_default()),
        &ctx,
    );
    let html = render(&campaign.html_text.clone().unwrap_or_default(), &ctx);
    let plain = render(&campaign.plain_text.clone().unwrap_or_default(), &ctx);

    let mailer: Box<dyn crate::services::mailer::EmailProvider> = brand
        .as_ref()
        .map(mailer_for_brand)
        .unwrap_or_else(|| Box::new(StubMailer));

    let attachments =
        crate::services::attachments::load_for_campaign(campaign_token.raw()).await;
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
        tracing::warn!(error = ?e, "test send failed");
        return redirect(&format!("/app/campaigns/{}/edit?e=test_send", campaign_token));
    }
    redirect(&format!("/app/campaigns/{}/edit?i=test_sent", campaign_token))
}

#[handler]
pub async fn send_now(
    _user: CurrentUser,
    Path(campaign_token): Path<OpaqueId<Campaign>>,
    Form(form): Form<SendForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let now = chrono::Utc::now().timestamp();
    let row = match campaigns::Entity::find_by_id(campaign_token.raw()).one(&state.db).await {
        Ok(Some(c)) => c,
        _ => return redirect("/app/brands"),
    };
    let mut active: campaigns::ActiveModel = row.into();
    active.sent = Set(Some(now.to_string()));
    active.to_send_lists = Set(Some(form.list_ids));
    active.send_date = Set(Some("0".into()));
    active.timezone = Set(Some("0".into()));
    let _ = active.update(&state.db).await;
    redirect(&format!("/app/campaigns/{}/edit", campaign_token))
}

#[handler]
pub async fn send_to(
    _user: CurrentUser,
    Path(campaign_token): Path<OpaqueId<Campaign>>,
    Form(form): Form<SendForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let now = chrono::Utc::now().timestamp();
    let row = match campaigns::Entity::find_by_id(campaign_token.raw()).one(&state.db).await {
        Ok(Some(c)) => c,
        _ => return redirect("/app/brands"),
    };
    let mut active: campaigns::ActiveModel = row.into();
    active.sent = Set(Some(now.to_string()));
    active.to_send_lists = Set(Some(form.list_ids));
    active.send_date = Set(Some("0".into()));
    active.timezone = Set(Some("0".into()));
    let _ = active.update(&state.db).await;
    redirect(&format!("/app/campaigns/{}/edit", campaign_token))
}

#[handler]
pub async fn stop(
    _user: CurrentUser,
    Path(campaign_token): Path<OpaqueId<Campaign>>,
    Data(state): Data<&AppState>,
) -> Response {
    let row = match campaigns::Entity::find_by_id(campaign_token.raw()).one(&state.db).await {
        Ok(Some(c)) => c,
        _ => return redirect("/app/brands"),
    };
    let mut active: campaigns::ActiveModel = row.into();
    active.campaign_stopped = Set(Some(1));
    let _ = active.update(&state.db).await;
    redirect(&format!("/app/campaigns/{}/edit", campaign_token))
}

#[handler]
pub async fn delete(
    _user: CurrentUser,
    Path(campaign_token): Path<OpaqueId<Campaign>>,
    Query(q): Query<BrandQuery>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = campaigns::Entity::delete_by_id(campaign_token.raw())
        .exec(&state.db)
        .await;
    redirect(&format!("/app/brands/{}/campaigns", q.brand))
}

#[handler]
pub async fn report(
    _user: CurrentUser,
    Path(campaign_token): Path<OpaqueId<Campaign>>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let row = match campaigns::Entity::find_by_id(campaign_token.raw()).one(&state.db).await {
        Ok(Some(c)) => c,
        _ => return redirect("/app/brands"),
    };
    let report = crate::services::reports::build(&state.db, row.id)
        .await
        .unwrap_or_else(|e| {
            tracing::error!(error = ?e, "report build failed");
            crate::services::reports::CampaignReport {
                recipients: 0, to_send: 0, unique_opens: 0, total_opens: 0,
                unique_clickers: 0, total_clicks: 0, unsubscribed: 0, bounced: 0, complained: 0,
                top_links: Vec::new(), unsubscribed_subs: Vec::new(),
                bounced_subs: Vec::new(), complained_subs: Vec::new(),
                top_countries: Vec::new(),
            }
        });
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Campaign report", move || {
        view! { <CampaignReportPage campaign=row report_data=report/> }
    })
    .await
}

/// `GET /app/campaigns/:id/report.csv` — single-campaign report download.
#[handler]
pub async fn report_csv(
    _user: CurrentUser,
    Path(campaign_token): Path<OpaqueId<Campaign>>,
    Data(state): Data<&AppState>,
) -> Response {
    let r = match crate::services::reports::build(&state.db, campaign_token.raw()).await {
        Ok(r) => r,
        Err(_) => return redirect(&format!("/app/campaigns/{}/report", campaign_token)),
    };
    let mut wtr = csv::Writer::from_writer(Vec::new());
    let _ = wtr.write_record(&["metric", "value"]);
    let _ = wtr.write_record(&["recipients", &r.recipients.to_string()]);
    let _ = wtr.write_record(&["to_send", &r.to_send.to_string()]);
    let _ = wtr.write_record(&["unique_opens", &r.unique_opens.to_string()]);
    let _ = wtr.write_record(&["total_opens", &r.total_opens.to_string()]);
    let _ = wtr.write_record(&["unique_clickers", &r.unique_clickers.to_string()]);
    let _ = wtr.write_record(&["total_clicks", &r.total_clicks.to_string()]);
    let _ = wtr.write_record(&["unsubscribed", &r.unsubscribed.to_string()]);
    let _ = wtr.write_record(&["bounced", &r.bounced.to_string()]);
    let _ = wtr.write_record(&["complained", &r.complained.to_string()]);
    let _ = wtr.write_record(&[] as &[&str]);
    let _ = wtr.write_record(&["link", "clicks"]);
    for l in &r.top_links {
        let _ = wtr.write_record(&[&l.url, &l.clicks.to_string()]);
    }
    let body = wtr.into_inner().unwrap_or_default();
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/csv; charset=utf-8")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"campaign-{}.csv\"", campaign_token),
        )
        .body(body)
}

/// `GET /app/campaigns/:id/preview` — render the campaign body with a
/// dummy subscriber context and return the resulting HTML wrapped in a
/// stand-alone preview document. Used by the "Preview" button in the
/// editor. `?view=plain` opens the plain-text pane on first paint.
#[handler]
pub async fn preview(
    _user: CurrentUser,
    Path(campaign_token): Path<OpaqueId<Campaign>>,
    Query(q): Query<PreviewQuery>,
    Data(state): Data<&AppState>,
) -> Response {
    use crate::services::personalization::Context;
    use crate::services::preview::{build_preview, dummy_subscriber, PreviewMeta, PreviewMode};

    let campaign = match campaigns::Entity::find_by_id(campaign_token.raw())
        .one(&state.db)
        .await
    {
        Ok(Some(c)) => c,
        _ => return redirect("/app/brands"),
    };

    let dummy = dummy_subscriber("preview@mailer.local", "Preview Recipient");
    let app_url = state.config.app_url.clone();
    let ctx = Context::build(&dummy, 0, campaign.id, "", &app_url);

    let from_name = campaign.from_name.clone().unwrap_or_default();
    let from_email = campaign.from_email.clone().unwrap_or_default();
    let subject = campaign.subject.clone().unwrap_or_default();
    let html_body = campaign.html_text.clone().unwrap_or_default();
    let plain_body = campaign.plain_text.clone().unwrap_or_default();
    let back_href = format!("/app/campaigns/{campaign_token}/edit");
    let mode = PreviewMode::from_query(q.view.as_deref());

    let html = build_preview(
        &html_body,
        &plain_body,
        mode,
        PreviewMeta {
            kind: "Campaign",
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
