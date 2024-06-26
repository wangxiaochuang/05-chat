use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

use crate::{
    error::{AppError, ErrorOutput},
    services::{CreateUser, SigninUser},
    AppState,
};

#[derive(Debug, ToSchema, Serialize, Deserialize)]
pub struct AuthOutput {
    token: String,
}

/// Create a new user in the chat system with email and password.
///
/// - If the email already exists, it will return 409.
/// - Otherwise, it will return 201 with a token.
/// - If the workspace doesn't exist, it will create one.
#[utoipa::path(
    post,
    path = "/api/signup",
    responses(
        (status = 200, description = "User created", body = AuthOutput),
        (status = 409, description = "User has exist", body = ErrorOutput),
    )
)]
pub(crate) async fn signup_handler(
    State(state): State<AppState>,
    Json(input): Json<CreateUser>,
) -> Result<impl IntoResponse, AppError> {
    let user = state.user_svc.create(&input).await?;
    let token = state.ek.sign(user)?;
    Ok((StatusCode::CREATED, Json(json!(AuthOutput { token }))))
}

/// sign user in the chat system with email and password.
///
/// - If success, it'll return 201 with a token.
#[utoipa::path(
    post,
    path = "/api/signin",
    responses(
        (status = 200, description = "login ok", body = AuthOutput),
    )
)]
pub(crate) async fn signin_handler(
    State(state): State<AppState>,
    Json(input): Json<SigninUser>,
) -> Result<impl IntoResponse, AppError> {
    let user = state.user_svc.verify(&input).await?;
    match user {
        Some(user) => {
            let token = state.ek.sign(user)?;
            Ok((StatusCode::OK, Json(json!(AuthOutput { token }))).into_response())
        }
        None => Ok((
            StatusCode::FORBIDDEN,
            Json(json!(ErrorOutput::new("Invalid email or password"))),
        )
            .into_response()),
    }
}

#[cfg(test)]
mod tests {
    use crate::{error::ErrorOutput, test_util::get_test_state_and_pg};

    use super::*;
    use anyhow::Result;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn signup_should_work() -> Result<()> {
        let (state, _tpg) = get_test_state_and_pg().await?;
        let input = CreateUser::new("none", "jack", "admin@admin.com", "Hunter42");
        let ret = signup_handler(State(state), Json(input))
            .await?
            .into_response();
        let body = ret.into_body().collect().await.unwrap().to_bytes();
        let auth: AuthOutput = serde_json::from_slice(&body)?;
        assert_ne!(auth.token, "");

        Ok(())
    }

    #[tokio::test]
    async fn signup_duplicate_user_should_409() -> Result<()> {
        let (state, _tpg) = get_test_state_and_pg().await?;
        let input = CreateUser::new("ws1", "jack1", "jack1@gmail.com", "Hunter42");
        let ret = signup_handler(State(state), Json(input))
            .await
            .into_response();
        assert_eq!(ret.status(), StatusCode::CONFLICT);
        let body = ret.into_body().collect().await.unwrap().to_bytes();
        let ret: ErrorOutput = serde_json::from_slice(&body)?;
        assert_eq!(ret.error, "email already exists: jack1@gmail.com");

        Ok(())
    }

    #[tokio::test]
    async fn duplicate_user_create_should_fail() -> Result<()> {
        let (state, _tpg) = get_test_state_and_pg().await?;
        let input = CreateUser::new("none", "jack", "admin@admin.com", "Hunter42");
        state.user_svc.create(&input).await?;
        let ret = state.user_svc.create(&input).await;
        match ret {
            Err(AppError::EmailAlreadyExists(email)) => assert_eq!(email, input.email),
            _ => panic!("should be duplicate user error"),
        }
        Ok(())
    }
    #[tokio::test]
    async fn signin_with_wrong_password_should_403() -> Result<()> {
        let (state, _tpg) = get_test_state_and_pg().await?;
        let input = SigninUser::new("jack1@gmail.com", "wrong-password");
        let ret = signin_handler(State(state.clone()), Json(input))
            .await?
            .into_response();
        assert_eq!(ret.status(), StatusCode::FORBIDDEN);
        let body = ret.into_body().collect().await.unwrap().to_bytes();
        let ret: ErrorOutput = serde_json::from_slice(&body)?;
        assert_eq!(ret.error, "Invalid email or password");
        Ok(())
    }

    #[tokio::test]
    async fn signin_with_non_exist_user_should_403() -> Result<()> {
        let (state, _tpg) = get_test_state_and_pg().await?;
        let input = SigninUser::new("non-exist@admin.com", "Hunter42");
        let ret = signin_handler(State(state.clone()), Json(input))
            .await?
            .into_response();
        assert_eq!(ret.status(), StatusCode::FORBIDDEN);
        let body = ret.into_body().collect().await.unwrap().to_bytes();
        let ret: ErrorOutput = serde_json::from_slice(&body)?;
        assert_eq!(ret.error, "Invalid email or password");
        Ok(())
    }

    #[tokio::test]
    async fn signin_should_work() -> Result<()> {
        let (state, _tpg) = get_test_state_and_pg().await?;
        let email = "jack1@gmail.com";
        let password = "Hunter48";

        let input = SigninUser::new(email, password);
        let ret = signin_handler(State(state.clone()), Json(input))
            .await?
            .into_response();
        assert_eq!(ret.status(), 200);
        let body = ret.into_body().collect().await.unwrap().to_bytes();
        let auth: AuthOutput = serde_json::from_slice(&body)?;
        assert_ne!(auth.token, "");
        Ok(())
    }
}
