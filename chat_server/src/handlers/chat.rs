use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use chat_core::User;

use crate::{
    error::AppError,
    services::{CreateChat, UpdateChat},
    AppState,
};

pub(crate) async fn list_chat_handler(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse, AppError> {
    let chats = state.chat_svc.fetch_all(user.ws_id as _).await?;
    Ok((StatusCode::OK, Json(chats)))
}

pub(crate) async fn create_chat_handler(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(input): Json<CreateChat>,
) -> Result<impl IntoResponse, AppError> {
    let chat = state.chat_svc.create(input, user.ws_id as _).await?;
    Ok((StatusCode::CREATED, Json(chat)))
}

pub(crate) async fn get_chat_handler(
    State(state): State<AppState>,
    Path(chat_id): Path<u64>,
) -> Result<impl IntoResponse, AppError> {
    let chat = state.chat_svc.get_by_id(chat_id).await?;
    let chat = match chat {
        Some(chat) => chat,
        None => return Err(AppError::NotFound("chat id not found".to_owned())),
    };
    Ok((StatusCode::OK, Json(chat)))
}

pub(crate) async fn update_chat_handler(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(chat_id): Path<u64>,
    Json(input): Json<UpdateChat>,
) -> Result<impl IntoResponse, AppError> {
    let chat = state
        .chat_svc
        .update(input, user.ws_id as _, chat_id)
        .await?;
    Ok((StatusCode::OK, Json(chat)))
}

pub(crate) async fn delete_chat_handler(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(chat_id): Path<u64>,
) -> Result<impl IntoResponse, AppError> {
    let chat = state.chat_svc.delete(user.ws_id as _, chat_id).await?;
    Ok((StatusCode::OK, Json(chat)))
}
