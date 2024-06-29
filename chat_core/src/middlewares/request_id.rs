use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use tracing::warn;
use uuid::Uuid;

use super::REQUEST_ID_HEADER;

pub async fn set_request_id(mut req: Request, next: Next) -> Response {
    let id = match req.headers().get(REQUEST_ID_HEADER) {
        Some(v) => Some(v.to_owned()),
        None => HeaderValue::from_str(&Uuid::now_v7().to_string())
            .map(|v| {
                req.headers_mut().insert(REQUEST_ID_HEADER, v.to_owned());
                v
            })
            .map_err(|e| {
                warn!("parse generated request id failed: {}", e);
                e
            })
            .ok(),
    };
    let mut resp = next.run(req).await;

    if let Some(id) = id {
        resp.headers_mut().insert(REQUEST_ID_HEADER, id);
    };

    resp
}
