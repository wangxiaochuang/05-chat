use axum::{extract::State, response::IntoResponse, Extension, Json};
use chat_core::User;

use crate::{error::AppError, AppState};

pub(crate) async fn list_chat_users_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let users = state.ws_svc.fetch_all_chat_users(user.ws_id as _).await?;
    Ok(Json(users))
}
