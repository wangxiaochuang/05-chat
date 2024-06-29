use axum::{
    extract::{Path, Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
    Extension,
};
use chat_core::User;

use crate::{error::AppError, AppState};

pub async fn verify_chat_perm(
    State(state): State<AppState>,
    Path(chat_id): Path<u64>,
    Extension(user): Extension<User>,
    req: Request,
    next: Next,
) -> Response {
    match state.chat_svc.is_chat_member(chat_id, user.id as _).await {
        Err(e) => return e.into_response(),
        Ok(is_member) if !is_member => return AppError::PermissionDeny.into_response(),
        _ => {}
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body, http::StatusCode, middleware::from_fn_with_state, routing::get, Router,
    };
    use chat_core::middlewares::verify_token_v2;
    use tower::ServiceExt;

    use crate::test_util::get_test_state_and_pg;

    use super::*;

    async fn handler() -> String {
        "hello".to_string()
    }

    #[tokio::test]
    async fn verify_chat_perm_middleware_should_work() {
        let (state, _pg) = get_test_state_and_pg().await.unwrap();
        let user = User::new(1, "jack", "jack@gmail.com");
        let token = state.ek.sign(user).expect("sign should work");

        let app = Router::new()
            .route("/:id", get(handler))
            .layer(from_fn_with_state(state.clone(), verify_chat_perm))
            .layer(from_fn_with_state(
                state.clone(),
                verify_token_v2::<AppState>,
            ))
            .with_state(state);

        let req = Request::builder()
            .uri("/4")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .expect("request builder");
        let res = app.clone().oneshot(req).await.expect("oneshot should work");
        assert_eq!(res.status(), StatusCode::OK);

        let req = Request::builder()
            .uri("/5")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .expect("request builder");
        let res = app.clone().oneshot(req).await.expect("oneshot should work");
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
    }
}
