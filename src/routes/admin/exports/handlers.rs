//! CSV export endpoints — subscribers per list, suppression list,
//! blocked domains, campaigns per brand.

use poem::{
    handler,
    http::StatusCode,
    web::{Data, Path},
    Response,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::database::entities::{blocked_domains, campaigns, subscribers, suppression_list};
use crate::routes::admin::session::CurrentUser;
use crate::services::lists;
use crate::utils::opaque_id::{Brand, List, OpaqueId};
use crate::AppState;

fn csv_response(filename: &str, body: Vec<u8>) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/csv; charset=utf-8")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{filename}\""),
        )
        .body(body)
}

/// `GET /app/lists/:id/subscribers/export.csv`
#[handler]
pub async fn subscribers_csv(
    _user: CurrentUser,
    Path(list_token): Path<OpaqueId<List>>,
    Data(state): Data<&AppState>,
) -> Response {
    let list = match lists::find_list(&state.db, list_token.raw()).await.ok().flatten() {
        Some(l) => l,
        None => return csv_response("subscribers.csv", b"email\n".to_vec()),
    };
    let cf_meta = lists::parse_custom_fields(list.custom_fields.as_deref().unwrap_or(""));

    let rows = subscribers::Entity::find()
        .filter(subscribers::Column::List.eq(list_token.raw()))
        .order_by_asc(subscribers::Column::Id)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let mut wtr = csv::Writer::from_writer(Vec::new());
    let mut header = vec!["email".to_string(), "name".to_string(), "status".to_string(), "country".to_string(), "joined".to_string()];
    for cf in &cf_meta {
        header.push(cf.name.clone());
    }
    let _ = wtr.write_record(&header);

    for s in rows {
        let status = if s.unsubscribed.unwrap_or(0) == 1 { "unsubscribed" }
            else if s.bounced.unwrap_or(0) == 1 { "bounced" }
            else if s.complaint.unwrap_or(0) == 1 { "complained" }
            else if s.confirmed.unwrap_or(0) == 1 { "subscribed" }
            else { "unconfirmed" };
        let joined = chrono::DateTime::<chrono::Utc>::from_timestamp(s.timestamp.unwrap_or(0), 0)
            .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();
        let cf_values: Vec<&str> = s.custom_fields.as_deref().unwrap_or("").split("%s%").collect();

        let mut row = vec![
            s.email.unwrap_or_default(),
            s.name.unwrap_or_default(),
            status.to_string(),
            s.country.unwrap_or_default(),
            joined,
        ];
        for (i, _) in cf_meta.iter().enumerate() {
            row.push(cf_values.get(i).copied().unwrap_or("").to_string());
        }
        let _ = wtr.write_record(&row);
    }

    let body = wtr.into_inner().unwrap_or_default();
    csv_response(&format!("subscribers-{}.csv", list_token), body)
}

/// `GET /app/brands/:id/suppression-list/export.csv`
#[handler]
pub async fn suppression_csv(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Data(state): Data<&AppState>,
) -> Response {
    let rows = suppression_list::Entity::find()
        .filter(suppression_list::Column::App.eq(brand_token.raw()))
        .order_by_asc(suppression_list::Column::Id)
        .all(&state.db)
        .await
        .unwrap_or_default();
    let mut wtr = csv::Writer::from_writer(Vec::new());
    let _ = wtr.write_record(&["email", "added", "block_attempts"]);
    for r in rows {
        let added = chrono::DateTime::<chrono::Utc>::from_timestamp(r.timestamp.unwrap_or(0), 0)
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default();
        let _ = wtr.write_record(&[
            r.email.unwrap_or_default(),
            added,
            r.block_attempts.unwrap_or(0).to_string(),
        ]);
    }
    let body = wtr.into_inner().unwrap_or_default();
    csv_response(&format!("suppression-{}.csv", brand_token), body)
}

/// `GET /app/brands/:id/blocked-domains/export.csv`
#[handler]
pub async fn blocked_domains_csv(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Data(state): Data<&AppState>,
) -> Response {
    let rows = blocked_domains::Entity::find()
        .filter(blocked_domains::Column::App.eq(brand_token.raw()))
        .order_by_asc(blocked_domains::Column::Id)
        .all(&state.db)
        .await
        .unwrap_or_default();
    let mut wtr = csv::Writer::from_writer(Vec::new());
    let _ = wtr.write_record(&["domain", "added", "block_attempts"]);
    for r in rows {
        let added = chrono::DateTime::<chrono::Utc>::from_timestamp(r.timestamp.unwrap_or(0), 0)
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default();
        let _ = wtr.write_record(&[
            r.domain.unwrap_or_default(),
            added,
            r.block_attempts.unwrap_or(0).to_string(),
        ]);
    }
    let body = wtr.into_inner().unwrap_or_default();
    csv_response(&format!("blocked-domains-{}.csv", brand_token), body)
}

/// `GET /app/brands/:id/campaigns/export.csv`
#[handler]
pub async fn campaigns_csv(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Data(state): Data<&AppState>,
) -> Response {
    let rows = campaigns::Entity::find()
        .filter(campaigns::Column::App.eq(brand_token.raw()))
        .order_by_asc(campaigns::Column::Id)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let mut wtr = csv::Writer::from_writer(Vec::new());
    let _ = wtr.write_record(&[
        "title", "subject", "from_name", "from_email", "sent",
        "to_send", "recipients", "status",
    ]);
    for c in rows {
        let sent_label = chrono::DateTime::<chrono::Utc>::from_timestamp(
            c.sent.as_deref().and_then(|s| s.parse().ok()).unwrap_or(0),
            0,
        )
        .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default();
        let status = if c.sent.as_deref().unwrap_or("").is_empty() {
            "draft"
        } else if c.to_send.unwrap_or(0) <= c.recipients.unwrap_or(0) {
            "sent"
        } else if c.send_date.is_some() {
            "scheduled"
        } else {
            "sending"
        };
        let _ = wtr.write_record(&[
            c.title.unwrap_or_default(),
            c.subject.unwrap_or_default(),
            c.from_name.unwrap_or_default(),
            c.from_email.unwrap_or_default(),
            sent_label,
            c.to_send.unwrap_or(0).to_string(),
            c.recipients.unwrap_or(0).to_string(),
            status.to_string(),
        ]);
    }
    let body = wtr.into_inner().unwrap_or_default();
    csv_response(&format!("campaigns-{}.csv", brand_token), body)
}

/// `GET /app/segments/:id/export.csv` — every active subscriber that
/// matches the segment.
#[handler]
pub async fn segment_members_csv(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Data(state): Data<&AppState>,
) -> Response {
    let list_id = crate::services::segments::find(&state.db, id)
        .await
        .ok()
        .flatten()
        .and_then(|s| s.list)
        .unwrap_or(0);
    let ids = crate::services::segments::get_matching_subscriber_ids(&state.db, id, list_id)
        .await
        .unwrap_or_default();
    if ids.is_empty() {
        return csv_response(
            &format!("segment-{id}-members.csv"),
            b"email,name,country\n".to_vec(),
        );
    }
    let rows = subscribers::Entity::find()
        .filter(subscribers::Column::Id.is_in(ids))
        .order_by_asc(subscribers::Column::Id)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let mut wtr = csv::Writer::from_writer(Vec::new());
    let _ = wtr.write_record(&["email", "name", "country", "joined"]);
    for s in rows {
        let joined = chrono::DateTime::<chrono::Utc>::from_timestamp(s.timestamp.unwrap_or(0), 0)
            .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();
        let _ = wtr.write_record(&[
            s.email.unwrap_or_default(),
            s.name.unwrap_or_default(),
            s.country.unwrap_or_default(),
            joined,
        ]);
    }
    let body = wtr.into_inner().unwrap_or_default();
    csv_response(&format!("segment-{id}-members.csv"), body)
}
