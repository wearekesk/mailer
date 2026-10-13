use poem::{
    http::{header, StatusCode},
    web::{Data, Json},
    Error, FromRequest, IntoResponse, Request, RequestBody, Result,
};

use crate::database::entities::login;
use crate::routes::api::ApiError;
use crate::services::auth;
use crate::AppState;

/// Header carrying the API key for every authenticated request.
pub const API_KEY_HEADER: &str = "X-Api-Key";

/// Authenticated caller. Extract this in any handler that requires a valid
/// API key — extraction fails with a typed JSON `401`/`500` response if the
/// `X-Api-Key` header is missing, malformed, or unknown.
pub struct ApiAuth(pub login::Model);

impl<'a> FromRequest<'a> for ApiAuth {
    async fn from_request(req: &'a Request, body: &mut RequestBody) -> Result<Self> {
        let key = req
            .headers()
            .get(API_KEY_HEADER)
            .or_else(|| req.headers().get(header::AUTHORIZATION).filter(|v| {
                v.to_str()
                    .map(|s| s.starts_with("Bearer "))
                    .unwrap_or(false)
            }))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.strip_prefix("Bearer ").unwrap_or(s).trim())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                json_err(StatusCode::UNAUTHORIZED, "Missing X-Api-Key header")
            })?;

        let state = <Data<&AppState>>::from_request(req, body).await?;

        let user = auth::lookup_user_by_api_key(&state.0.db, key)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "lookup_user_by_api_key failed");
                json_err(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
            })?
            .ok_or_else(|| json_err(StatusCode::UNAUTHORIZED, "Invalid API key"))?;

        Ok(ApiAuth(user))
    }
}

fn json_err(status: StatusCode, msg: &str) -> Error {
    let mut resp = Json(ApiError::new(msg)).into_response();
    resp.set_status(status);
    Error::from_response(resp)
}

/// Middleware for routes requiring an API key.
pub struct ApiKeyAuthMiddleware;

impl<E: poem::Endpoint> poem::Middleware<E> for ApiKeyAuthMiddleware {
    type Output = ApiKeyAuthEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        ApiKeyAuthEndpoint(ep)
    }
}

pub struct ApiKeyAuthEndpoint<E>(E);

impl<E: poem::Endpoint> poem::Endpoint for ApiKeyAuthEndpoint<E> {
    type Output = poem::Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let (mut req, mut body) = req.split();
        let auth = match ApiAuth::from_request(&req, &mut body).await {
            Ok(auth) => auth,
            Err(e) => return Ok(e.into_response()),
        };
        if let Ok(body) = body.take() {
            req.set_body(body);
        }
        req.extensions_mut().insert(auth.0);
        let resp = self.0.call(req).await?.into_response();
        Ok(resp)
    }
}
