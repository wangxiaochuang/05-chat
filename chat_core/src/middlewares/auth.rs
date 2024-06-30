use axum::{
    extract::{FromRequestParts, Query, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use serde::Deserialize;
use tracing::warn;

use super::TokenVerify;

#[allow(dead_code)]
pub async fn verify_token<T>(State(state): State<T>, req: Request, next: Next) -> Response
where
    T: TokenVerify + Clone + Send + Sync + 'static,
{
    let (mut parts, body) = req.into_parts();
    let req =
        match TypedHeader::<Authorization<Bearer>>::from_request_parts(&mut parts, &state).await {
            Ok(TypedHeader(Authorization(bearer))) => {
                let token = bearer.token();
                match state.verify_token(token) {
                    Ok(user) => {
                        let mut req = Request::from_parts(parts, body);
                        req.extensions_mut().insert(user);
                        req
                    }
                    Err(e) => {
                        let msg = format!("verify token failed: {:?}", e);
                        warn!(msg);
                        return (StatusCode::FORBIDDEN, msg).into_response();
                    }
                }
            }
            Err(e) => {
                let msg = format!("parse Authorization header failed: {}", e);
                warn!(msg);
                return (StatusCode::UNAUTHORIZED, msg).into_response();
            }
        };
    next.run(req).await
}
#[derive(Debug, Deserialize)]
pub struct AuthInfo {
    pub token: String,
}
pub async fn verify_token_v2<T>(
    State(state): State<T>,
    bearer: Option<TypedHeader<axum_extra::headers::Authorization<Bearer>>>,
    query: Option<Query<AuthInfo>>,
    mut req: Request,
    next: Next,
) -> Response
where
    T: TokenVerify + Clone + Send + Sync + 'static,
{
    let token = match (&bearer, &query) {
        (Some(TypedHeader(bearer)), _) => bearer.token(),
        (_, Some(Query(AuthInfo { ref token }))) => token,
        _ => return (StatusCode::BAD_REQUEST, "need token").into_response(),
    };
    match state.verify_token(token) {
        Ok(user) => {
            req.extensions_mut().insert(user);
        }
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                format!("parse Authorization header failed: {:?}", e),
            )
                .into_response()
        }
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        utils::{DecodingKey, EncodingKey},
        User,
    };
    use anyhow::Result;
    use axum::{body::Body, http::Request, middleware::from_fn_with_state, routing::get, Router};
    use tower::ServiceExt;

    #[derive(Clone)]
    struct AppState(Arc<AppStateInner>);
    struct AppStateInner {
        dk: DecodingKey,
        ek: EncodingKey,
    }

    impl TokenVerify for AppState {
        type Error = anyhow::Error;
        fn verify_token(&self, token: &str) -> Result<User> {
            self.0.dk.verify(token)
        }
    }

    async fn handler() -> String {
        "hello".to_string()
    }

    #[tokio::test]
    async fn verify_token_middleware_should_work() -> Result<()> {
        let encoding_pem = include_str!("../../fixtures/encoding.pem");
        let decoding_pem = include_str!("../../fixtures/decoding.pem");

        let ek = EncodingKey::load(encoding_pem)?;
        let dk = DecodingKey::load(decoding_pem)?;
        let state = AppState(Arc::new(AppStateInner { dk, ek }));
        let user = User::new(1, "jack", "jack@admin");
        let token = state.0.ek.sign(user)?;

        let app = Router::new()
            .route("/", get(handler))
            .layer(from_fn_with_state(
                state.clone(),
                verify_token_v2::<AppState>,
            ))
            .with_state(state);

        // with good token in authorization
        let req = Request::builder()
            .uri("/")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;
        let res = app.clone().oneshot(req).await?;
        assert_eq!(res.status(), StatusCode::OK);

        // with good token in query
        let req = Request::builder()
            .uri(format!("/?token={}", token))
            .body(Body::empty())?;
        let res = app.clone().oneshot(req).await?;
        assert_eq!(res.status(), StatusCode::OK);

        // no token
        let req = Request::builder().uri("/").body(Body::empty())?;
        let res = app.clone().oneshot(req).await?;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        // bad token
        let req = Request::builder()
            .uri("/")
            .header("Authorization", "Bearer bad-token")
            .body(Body::empty())?;
        let res = app.oneshot(req).await?;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        Ok(())
    }
}
