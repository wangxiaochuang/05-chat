use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
        HeaderMap, StatusCode,
    },
    response::IntoResponse,
    Extension, Json,
};
use chat_core::{Message, User};
use tokio::fs;
use tokio_util::io::ReaderStream;
use tracing::{info, warn};

use crate::{
    error::AppError,
    models::ChatFile,
    services::{CreateMessage, ListMessageOption},
    AppState,
};

pub(crate) async fn send_message_handler(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(chat_id): Path<u64>,
    Json(input): Json<CreateMessage>,
) -> Result<impl IntoResponse, AppError> {
    let message = state.msg_svc.create(input, chat_id, user.id as _).await?;
    Ok((StatusCode::CREATED, Json(message)))
}

pub(crate) async fn list_message_handler(
    State(state): State<AppState>,
    Path(chat_id): Path<u64>,
    Query(input): Query<ListMessageOption>,
) -> Result<impl IntoResponse, AppError> {
    let messages: Vec<Message> = state.msg_svc.list(input, chat_id as _).await?;
    Ok(Json(messages))
}

pub(crate) async fn file_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Path((ws_id, path)): Path<(u64, String)>,
) -> Result<impl IntoResponse, AppError> {
    if ws_id != user.ws_id as u64 {
        return Err(AppError::PermissionDeny);
    }

    let base_dir = state.config.server.base_dir.join(ws_id.to_string());
    let path = base_dir.join(path);
    if !path.exists() {
        return Err(AppError::NotFound("file doesn't exist".to_string()));
    }
    // get path filename
    let filename = path
        .file_name()
        .ok_or(AppError::AnyError(anyhow::anyhow!("invalid path")))?
        .to_str()
        .ok_or(AppError::AnyError(anyhow::anyhow!("invalid path")))?;
    let mime = mime_guess::from_path(&path).first_or_octet_stream();

    let file = fs::File::open(&path).await?;
    let stream = ReaderStream::new(file);
    // let body = fs::read(path).await?;
    let headers = HeaderMap::from_iter([
        (CONTENT_TYPE, mime.to_string().parse().unwrap()),
        (
            CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename)
                .parse()
                .unwrap(),
        ),
    ]);
    Ok((headers, Body::from_stream(stream)))
}

pub(crate) async fn upload_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let ws_id = user.ws_id as u64;
    let base_dir = &state.config.server.base_dir;
    let mut files = vec![];
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::AnyError(anyhow::anyhow!("multipart error")))?
    {
        let filename = field.file_name().map(|name| name.to_owned());
        let (Some(filename), Ok(data)) = (filename, field.bytes().await) else {
            warn!("failed to read multipart field");
            continue;
        };

        let file = ChatFile::new(ws_id, &filename, &data);
        files.push(file.url());
        let path = file.path(base_dir);
        if path.exists() {
            info!("File {} already exists: {:?}", filename, path);
            continue;
        } else {
            fs::create_dir_all(path.parent().expect("file path parent should exists")).await?;
            fs::write(path, data).await?;
        }
    }
    Ok(Json(files))
}
