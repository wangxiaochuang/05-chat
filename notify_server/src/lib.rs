use std::{ops::Deref, sync::Arc};

use axum::{
    middleware::from_fn_with_state,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use chat_core::{
    middlewares::{verify_token_v2, TokenVerify},
    utils::DecodingKey,
    User,
};
use config::AppConfig;
use dashmap::DashMap;
use error::AppError;
use notif::AppEvent;
use sse::sse_handler;
pub mod config;
mod error;
mod notif;
mod sse;
pub use notif::setup_pg_listener;
use tokio::sync::broadcast;

pub type UserMap = Arc<DashMap<u64, broadcast::Sender<Arc<AppEvent>>>>;

const INDEX_HTML: &str = include_str!("../index.html");

#[derive(Clone)]
pub struct AppState(Arc<AppStateInner>);

#[allow(unused)]
pub struct AppStateInner {
    pub(crate) config: AppConfig,
    users: UserMap,
    dk: DecodingKey,
}

impl Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let dk = DecodingKey::load(&config.auth.pk).expect("Failed to load public key");
        let users = Arc::new(DashMap::new());
        Self(Arc::new(AppStateInner { config, dk, users }))
    }
}

impl TokenVerify for AppState {
    type Error = AppError;
    fn verify_token(&self, token: &str) -> Result<User, AppError> {
        Ok(self.dk.verify(token)?)
    }
}

pub async fn get_router(config: AppConfig) -> anyhow::Result<Router> {
    let state = AppState::new(config);
    setup_pg_listener(state.clone()).await?;
    Ok(Router::new()
        .route("/events", get(sse_handler))
        .layer(from_fn_with_state(
            state.clone(),
            verify_token_v2::<AppState>,
        ))
        .route("/", get(index_handler))
        .with_state(state.clone()))
}

async fn index_handler() -> impl IntoResponse {
    Html(INDEX_HTML)
}
