//! Background campaign scheduler.
//!
//! Spawned by `main` as a tokio task. Every [`TICK_INTERVAL`] seconds:
//!
//! 1. Acquires a "cron lock" by atomically flipping `login.cron` from 0 to 1.
//! 2. Queries for campaigns that are either past their scheduled `send_date`
//!    or partially-sent (`to_send > recipients`).
//! 3. For each, walks the subscriber set in batches and dispatches emails
//!    via the brand's SMTP transport (or [`StubMailer`] when the brand
//!    has no SMTP configured). Each email body is personalized + has its
//!    links rewritten to point through the click-tracker before send.
//! 4. Updates `campaigns.recipients` after each send and watches
//!    `campaigns.campaign_stopped` so an admin can abort a send mid-flight.
//! 5. Releases the cron lock on the way out.

use std::sync::Arc;
use std::time::Duration;

use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, Condition, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect,
};

use crate::database::entities::{apps, campaigns, lists, login, subscribers};
use crate::services::mailer::{mailer_for_brand, EmailProvider, OutboundEmail, StubMailer};
use crate::services::personalization::{self, Context};
use crate::utils::errors::AppResult;

const TICK_INTERVAL: Duration = Duration::from_secs(30);
const SEND_BATCH_PER_TICK: u64 = 50;

/// Spawn the scheduler. The mailer arg is kept for tests/integration but the
/// production path resolves a per-brand mailer per campaign — see
/// [`mailer_for_brand`].
pub fn spawn(
    db: DatabaseConnection,
    fallback_mailer: Arc<dyn EmailProvider>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tracing::info!("Campaign scheduler started (tick = {:?})", TICK_INTERVAL);
        loop {
            if let Err(e) = tick(&db, fallback_mailer.as_ref()).await {
                tracing::error!(error = ?e, "scheduler tick failed");
            }
            tokio::time::sleep(TICK_INTERVAL).await;
        }
    })
}

async fn tick(db: &DatabaseConnection, fallback: &dyn EmailProvider) -> AppResult<()> {
    if !try_acquire_lock(db).await? {
        return Ok(());
    }
    let result = process_pending(db, fallback).await;
    release_lock(db).await?;
    result
}

async fn try_acquire_lock(db: &DatabaseConnection) -> AppResult<bool> {
    let main = login::Entity::find()
        .order_by_asc(login::Column::Id)
        .one(db)
        .await?;
    let Some(main) = main else { return Ok(false) };
    if main.cron.unwrap_or(0) == 1 {
        return Ok(false);
    }
    let mut active: login::ActiveModel = main.into();
    active.cron = Set(Some(1));
    active.update(db).await?;
    Ok(true)
}

async fn release_lock(db: &DatabaseConnection) -> AppResult<()> {
    let main = login::Entity::find()
        .order_by_asc(login::Column::Id)
        .one(db)
        .await?;
    let Some(main) = main else { return Ok(()) };
    let mut active: login::ActiveModel = main.into();
    active.cron = Set(Some(0));
    active.update(db).await?;
    Ok(())
}

/// Admin "Reset cron lock" button: forces `login.cron = 0` so a stuck scheduler
/// tick can be unstuck without restarting the server.
pub async fn force_release_lock(db: &DatabaseConnection) -> AppResult<()> {
    release_lock(db).await
}

async fn process_pending(db: &DatabaseConnection, fallback: &dyn EmailProvider) -> AppResult<()> {
    let now = chrono::Utc::now().timestamp();

    let due = campaigns::Entity::find()
        .filter(
            Condition::any()
                .add(
                    Condition::all()
                        .add(campaigns::Column::SendDate.is_not_null())
                        .add(campaigns::Column::Lists.is_not_null())
                        .add(campaigns::Column::Timezone.is_not_null()),
                )
                .add(
                    Condition::all().add(
                        sea_orm::sea_query::Expr::col(campaigns::Column::ToSend)
                            .gt(sea_orm::sea_query::Expr::col(campaigns::Column::Recipients)),
                    ),
                ),
        )
        .order_by_desc(campaigns::Column::Sent)
        .all(db)
        .await?;

    for campaign in due {
        if let Err(e) = run_campaign(db, fallback, &campaign, now).await {
            tracing::error!(campaign = campaign.id, error = ?e, "campaign send failed");
        }
    }

    // Autoresponders.
    if let Err(e) = process_autoresponders(db, now).await {
        tracing::error!(error = ?e, "autoresponder tick failed");
    }
    Ok(())
}

/// Walk every enabled autoresponder email and dispatch sends to subscribers
/// whose join_date + time_condition is in the past and who haven't already
/// received this email (`subscribers.last_ares < ares_emails.id`).
async fn process_autoresponders(db: &DatabaseConnection, now: i64) -> AppResult<()> {
    use crate::database::entities::{ares, ares_emails, lists};

    let emails = crate::services::autoresponders::enabled_emails(db).await?;
    let app_url = std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000".into());

    for email in emails {
        let Some(ares_id) = email.ares_id else { continue };
        let Some(parent) = ares::Entity::find_by_id(ares_id).one(db).await? else {
            continue;
        };
        let Some(list_id) = parent.list else { continue };
        let Some(list) = lists::Entity::find_by_id(list_id).one(db).await? else {
            continue;
        };
        let brand_id = list.app.unwrap_or_default();
        let brand = if brand_id != 0 {
            apps::Entity::find_by_id(brand_id).one(db).await?
        } else {
            None
        };

        let cf_packed = list.custom_fields.clone().unwrap_or_default();
        let mailer: Box<dyn EmailProvider> = brand
            .as_ref()
            .map(crate::services::mailer::mailer_for_brand)
            .unwrap_or_else(|| Box::new(crate::services::mailer::StubMailer));

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
            .or_else(|| Some(from_email.clone()))
            .unwrap_or_default();

        let subject_template = email.title.clone().unwrap_or_default();
        let html_template = email.html_text.clone().unwrap_or_default();
        let plain_template = email.plain_text.clone().unwrap_or_default();

        let offset_secs = crate::services::autoresponders::parse_time_condition(
            email.time_condition.as_deref().unwrap_or(""),
        )
        .unwrap_or(0);

        // Eligible subscribers: confirmed, active, on this list, joined long
        // enough ago, and have NOT already received this exact email.
        let cutoff = now - offset_secs;
        let mut candidates = subscribers::Entity::find()
            .filter(subscribers::Column::List.eq(list_id))
            .filter(subscribers::Column::Unsubscribed.eq(0i16))
            .filter(subscribers::Column::Bounced.eq(0i16))
            .filter(subscribers::Column::Complaint.eq(0i16))
            .filter(subscribers::Column::Confirmed.eq(1i16))
            .filter(subscribers::Column::JoinDate.lte(cutoff))
            .order_by_asc(subscribers::Column::Id)
            .limit(SEND_BATCH_PER_TICK)
            .all(db)
            .await?;

        // Honor `ares_emails.segs` (include) and `segs_excl` — both are
        // CSVs of seg ids. A non-empty include list narrows candidates to
        // the union of segment members; the exclude list removes anyone
        // matching either segment.
        let segs_in = parse_csv(email.segs.as_deref().unwrap_or(""));
        let segs_out = parse_csv(email.segs_excl.as_deref().unwrap_or(""));
        if !segs_in.is_empty() || !segs_out.is_empty() {
            use std::collections::HashSet;
            let mut include: Option<HashSet<i32>> = None;
            for &seg_id in &segs_in {
                let ids = crate::services::segments::get_matching_subscriber_ids(
                    db, seg_id, list_id,
                )
                .await
                .unwrap_or_default();
                let set: HashSet<i32> = ids.into_iter().collect();
                include = Some(match include {
                    None => set,
                    Some(prev) => prev.union(&set).copied().collect(),
                });
            }
            let mut exclude: HashSet<i32> = HashSet::new();
            for &seg_id in &segs_out {
                let ids = crate::services::segments::get_matching_subscriber_ids(
                    db, seg_id, list_id,
                )
                .await
                .unwrap_or_default();
                exclude.extend(ids);
            }
            candidates.retain(|s| {
                let included = include.as_ref().map(|i| i.contains(&s.id)).unwrap_or(true);
                included && !exclude.contains(&s.id)
            });
        }

        // Pre-load AR-email attachments once per email so each send clones
        // the bytes instead of re-reading the directory.
        let ares_attachments =
            crate::services::attachments::load_for_ares_email(email.id).await;

        for sub in candidates {
            // Skip if already received this autoresponder email.
            if sub.last_ares.unwrap_or(0) >= email.id {
                continue;
            }
            let Some(to_email) = sub.email.clone().filter(|s| !s.is_empty()) else {
                continue;
            };

            let ctx = Context::build(&sub, list_id, 0, &cf_packed, &app_url);
            let mut rendered_html = personalization::render(&html_template, &ctx);
            let rendered_plain = personalization::render(&plain_template, &ctx);
            let rendered_subject = personalization::render(&subject_template, &ctx);

            if email.links_tracking.unwrap_or(1) != 0 {
                if let Ok(h) =
                    personalization::rewrite_links_for_send(db, &rendered_html, &ctx).await
                {
                    rendered_html = h;
                }
            }
            if email.opens_tracking.unwrap_or(1) != 0 {
                rendered_html = personalization::insert_open_pixel(&rendered_html, &ctx);
            }

            let outbound = OutboundEmail {
                from_name: from_name.clone(),
                from_email: from_email.clone(),
                to_email,
                to_name: sub.name.clone(),
                reply_to: reply_to.clone(),
                subject: rendered_subject,
                html: rendered_html,
                plain: rendered_plain,
                attachments: ares_attachments.clone(),
            };

            if let Err(e) = mailer.send(outbound).await {
                tracing::warn!(error = ?e, ares_email = email.id, subscriber = sub.id, "ares send failed");
                continue;
            }

            // Mark this subscriber as having received this email.
            let mut active: subscribers::ActiveModel = sub.into();
            active.last_ares = Set(Some(email.id));
            active.update(db).await?;

            // Increment the autoresponder email's recipients counter.
            ares_emails::Entity::update_many()
                .col_expr(
                    ares_emails::Column::Recipients,
                    sea_orm::sea_query::Expr::col(ares_emails::Column::Recipients).add(1),
                )
                .filter(ares_emails::Column::Id.eq(email.id))
                .exec(db)
                .await?;
        }
    }
    Ok(())
}

async fn run_campaign(
    db: &DatabaseConnection,
    fallback: &dyn EmailProvider,
    campaign: &campaigns::Model,
    now: i64,
) -> AppResult<()> {
    if let Some(c) = campaigns::Entity::find_by_id(campaign.id).one(db).await? {
        if c.campaign_stopped.unwrap_or(0) == 1 {
            tracing::info!(campaign = campaign.id, "campaign stopped — skipping");
            return Ok(());
        }
    }

    let send_date_in_past = campaign
        .send_date
        .as_deref()
        .and_then(|s| s.parse::<i64>().ok())
        .map(|t| t <= now)
        .unwrap_or(false);
    let already_sending = campaign.to_send.unwrap_or(0) > campaign.recipients.unwrap_or(0);
    if !send_date_in_past && !already_sending {
        return Ok(());
    }

    if !already_sending {
        let mut active: campaigns::ActiveModel = campaign.clone().into();
        active.sent = Set(Some(now.to_string()));
        active.send_date = Set(None);
        active.lists = Set(None);
        active.timezone = Set(None);
        active.update(db).await?;
    }

    // Resolve the brand and pick the right mailer.
    let brand = match campaign.app {
        Some(app_id) => apps::Entity::find_by_id(app_id).one(db).await?,
        None => None,
    };
    let mailer: Box<dyn EmailProvider> = brand
        .as_ref()
        .map(mailer_for_brand)
        .unwrap_or_else(|| Box::new(StubMailer));
    let _ = fallback; // kept in scope for tests; production uses brand mailer

    let from_name = campaign.from_name.clone().unwrap_or_default();
    let from_email = campaign.from_email.clone().unwrap_or_default();
    let reply_to = campaign.reply_to.clone().unwrap_or(from_email.clone());
    let subject = campaign.subject.clone().unwrap_or_default();
    let html_template = campaign.html_text.clone().unwrap_or_default();
    let plain_template = campaign.plain_text.clone().unwrap_or_default();
    let raw_lists = campaign
        .to_send_lists
        .as_deref()
        .filter(|s| !s.is_empty())
        .or(campaign.lists.as_deref())
        .unwrap_or_default();
    let list_ids = parse_csv(raw_lists);
    let segment_ids = parse_csv(&campaign.segs.clone().unwrap_or_default());
    let excl_list_ids = parse_csv(&campaign.lists_excl.clone().unwrap_or_default());
    let excl_segment_ids = parse_csv(&campaign.segs_excl.clone().unwrap_or_default());
    if list_ids.is_empty() && segment_ids.is_empty() {
        return Ok(());
    }

    let gdpr_only = brand
        .as_ref()
        .and_then(|b| b.gdpr_only)
        .map(|v| v == 1)
        .unwrap_or(false);

    // Resolve the full recipient set (lists ∪ segments, minus excludes) once
    // and skip past everyone we've already sent to. Identical resolution to
    // services::campaigns::count_recipients so the displayed `to_send` and
    // who actually receives mail can never disagree.
    let recipient_set = crate::services::campaigns::resolve_recipient_ids(
        db,
        &list_ids,
        &segment_ids,
        &excl_list_ids,
        &excl_segment_ids,
        gdpr_only,
    )
    .await?;

    let already_sent = campaign.recipients.unwrap_or(0) as usize;

    // Fully sent — fire campaign_sent rule and bail out.
    if already_sent >= recipient_set.len() {
        if let Some(b) = brand.as_ref() {
            let _ = crate::services::rules::dispatch_campaign(
                db,
                b.id,
                campaign,
                "campaign_sent",
            )
            .await;
        }
        return Ok(());
    }

    let batch_ids: Vec<i32> = recipient_set
        .iter()
        .skip(already_sent)
        .take(SEND_BATCH_PER_TICK as usize)
        .map(|(id, _)| *id)
        .collect();

    let recipients = subscribers::Entity::find()
        .filter(subscribers::Column::Id.is_in(batch_ids.iter().copied()))
        .order_by_asc(subscribers::Column::Id)
        .all(db)
        .await?;

    // Pre-load custom_fields per list so personalization can resolve tags
    // without an extra query per recipient.
    let unique_lists: std::collections::HashSet<i32> =
        recipients.iter().filter_map(|s| s.list).collect();
    let list_custom_fields =
        load_list_custom_fields(db, &unique_lists.iter().copied().collect::<Vec<_>>()).await?;

    let app_url = std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000".into());

    let campaign_attachments =
        crate::services::attachments::load_for_campaign(campaign.id).await;

    // First batch — fire campaign_sending rule.
    if already_sent == 0 {
        if let Some(b) = brand.as_ref() {
            let _ = crate::services::rules::dispatch_campaign(
                db,
                b.id,
                campaign,
                "campaign_sending",
            )
            .await;
        }
    }

    for sub in recipients {
        let to_email = match sub.email.clone() {
            Some(e) if !e.is_empty() => e,
            _ => continue,
        };

        let list_id = sub.list.unwrap_or(0);
        let cf_packed = list_custom_fields
            .get(&list_id)
            .cloned()
            .unwrap_or_default();

        let ctx = Context::build(&sub, list_id, campaign.id, &cf_packed, &app_url);

        // Render personalization tags first, then rewrite links so that any
        // tags inside hrefs (e.g. [unsubscribe]) resolve to URLs before the
        // tracker rewrites them.
        let mut rendered_html = personalization::render(&html_template, &ctx);
        let rendered_plain = personalization::render(&plain_template, &ctx);
        let rendered_subject = personalization::render(&subject, &ctx);

        if campaign.links_tracking.unwrap_or(1) != 0 {
            match personalization::rewrite_links_for_send(db, &rendered_html, &ctx).await {
                Ok(h) => rendered_html = h,
                Err(e) => {
                    tracing::warn!(error = ?e, "link rewrite failed; sending raw html");
                }
            }
        }
        if campaign.opens_tracking.unwrap_or(1) != 0 {
            rendered_html = personalization::insert_open_pixel(&rendered_html, &ctx);
        }

        let outbound = OutboundEmail {
            from_name: from_name.clone(),
            from_email: from_email.clone(),
            to_email,
            to_name: sub.name.clone(),
            reply_to: reply_to.clone(),
            subject: rendered_subject,
            html: rendered_html,
            plain: rendered_plain,
            attachments: campaign_attachments.clone(),
        };

        if let Err(e) = mailer.send(outbound).await {
            tracing::warn!(error = ?e, subscriber = sub.id, "send failed");
            continue;
        }

        campaigns::Entity::update_many()
            .col_expr(
                campaigns::Column::Recipients,
                sea_orm::sea_query::Expr::expr(
                    sea_orm::sea_query::Func::coalesce([
                        sea_orm::sea_query::Expr::col(campaigns::Column::Recipients).into(),
                        sea_orm::sea_query::Expr::val(0).into(),
                    ])
                )
                .add(1),
            )
            .filter(campaigns::Column::Id.eq(campaign.id))
            .exec(db)
            .await?;

        // Bail mid-batch if an admin clicks "stop".
        if let Ok(Some(c)) = campaigns::Entity::find_by_id(campaign.id).one(db).await {
            if c.campaign_stopped.unwrap_or(0) == 1 {
                tracing::info!(campaign = campaign.id, "campaign stopped mid-send");
                return Ok(());
            }
        }
    }

    Ok(())
}

async fn load_list_custom_fields(
    db: &DatabaseConnection,
    list_ids: &[i32],
) -> AppResult<std::collections::HashMap<i32, String>> {
    let mut out = std::collections::HashMap::new();
    let rows = lists::Entity::find()
        .filter(lists::Column::Id.is_in(list_ids.iter().copied()))
        .all(db)
        .await?;
    for row in rows {
        out.insert(row.id, row.custom_fields.unwrap_or_default());
    }
    Ok(out)
}

fn parse_csv(raw: &str) -> Vec<i32> {
    raw.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != "0")
        .filter_map(|s| s.parse::<i32>().ok())
        .collect()
}
