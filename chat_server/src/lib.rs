use std::{fmt, ops::Deref, sync::Arc, time::Duration};

use anyhow::Context;
use axum::{
    middleware::from_fn_with_state,
    routing::{get, post},
    Router,
};
use chat_core::{
    middlewares::{set_layer, verify_token_v2, TokenVerify},
    utils::{DecodingKey, EncodingKey},
    User,
};
use config::{AppConfig, AuthConfig};
use error::AppError;
use handlers::{
    create_chat_handler, delete_chat_handler, file_handler, get_chat_handler, index_handler,
    list_chat_handler, list_chat_users_handler, list_message_handler, send_message_handler,
    signin_handler, signup_handler, update_chat_handler, upload_handler,
};

pub mod config;
mod error;
mod handlers;
mod middlewares;
mod models;
mod openapi;
mod services;

use middlewares::verify_chat_perm;
use openapi::OpenApiRouter;
use services::{ChatService, MsgService, UserService, WsService};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::fs;
#[derive(Debug, Clone)]
pub struct AppState {
    pub inner: Arc<AppStateInner>,
}

#[allow(unused)]
pub struct AppStateInner {
    pub config: AppConfig,
    pub(crate) ek: EncodingKey,
    pub(crate) dk: DecodingKey,
    pub(crate) pool: PgPool,
    pub(crate) chat_svc: ChatService,
    pub(crate) user_svc: UserService,
    pub(crate) ws_svc: WsService,
    pub(crate) msg_svc: MsgService,
}

impl TokenVerify for AppState {
    type Error = AppError;
    fn verify_token(&self, token: &str) -> Result<User, Self::Error> {
        Ok(self.dk.verify(token)?)
    }
}
pub async fn get_router(state: AppState) -> Result<Router, AppError> {
    // let state = AppState::try_new(config).await?;

    let chat_route = Router::new()
        .route(
            "/:id",
            get(get_chat_handler)
                .patch(update_chat_handler)
                .delete(delete_chat_handler)
                .post(send_message_handler),
        )
        .route("/:id/message", get(list_message_handler))
        .layer(from_fn_with_state(state.clone(), verify_chat_perm))
        .route("/", get(list_chat_handler).post(create_chat_handler));
    let api = Router::new()
        .route("/users", get(list_chat_users_handler))
        .nest("/chats", chat_route)
        .route("/upload", post(upload_handler))
        .route("/files/:ws_id/*path", get(file_handler))
        .layer(from_fn_with_state(
            state.clone(),
            verify_token_v2::<AppState>,
        ))
        .route("/signin", post(signin_handler))
        .route("/signup", post(signup_handler));

    let app = Router::new()
        .openapi()
        .route("/", get(index_handler))
        .nest("/api", api)
        .with_state(state);
    Ok(set_layer(app))
}

impl Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl AppState {
    fn load_key(conf: &AuthConfig) -> Result<(EncodingKey, DecodingKey), AppError> {
        let dk = DecodingKey::load(&conf.pk).context("load pk failed")?;
        let ek = EncodingKey::load(&conf.sk).context("load sk failed")?;
        Ok((ek, dk))
    }
    pub async fn try_new(config: AppConfig) -> Result<Self, AppError> {
        fs::create_dir_all(&config.server.base_dir)
            .await
            .context("create base_dir failed")?;
        let (ek, dk) = Self::load_key(&config.auth)?;
        let pool = PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(1000))
            .connect(&config.server.db_url)
            .await
            .context("connect db failed")?;
        let ws_svc = WsService::new(pool.clone());
        let user_svc = UserService::new(pool.clone(), ws_svc.clone());
        let chat_svc = ChatService::new(pool.clone(), user_svc.clone());
        let msg_svc = MsgService::new(pool.clone(), config.server.base_dir.clone());
        Ok(Self {
            inner: Arc::new(AppStateInner {
                config,
                ek,
                dk,
                pool,
                chat_svc,
                user_svc,
                ws_svc,
                msg_svc,
            }),
        })
    }
}

impl fmt::Debug for AppStateInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppStateInner")
            .field("config", &self.config)
            .finish()
    }
}

#[cfg(feature = "test-util")]
pub mod test_util {
    use std::sync::Arc;

    use anyhow::Result;
    use sqlx::Executor;
    use sqlx::PgPool;
    use sqlx_db_tester::TestPg;

    use crate::services::ChatService;
    use crate::services::MsgService;
    use crate::services::UserService;
    use crate::services::WsService;
    use crate::{config::AppConfig, error::AppError, AppState, AppStateInner};

    impl AppState {
        pub async fn try_test_new(
            config: AppConfig,
        ) -> Result<(Self, sqlx_db_tester::TestPg), AppError> {
            let (ek, dk) = Self::load_key(&config.auth)?;
            // let server_db_url = config.server.db_url.rsplitn(2, '/').skip(1).next().unwrap();
            let (server_db_url, _) = config.server.db_url.rsplit_once('/').unwrap();
            let (tdb, pool) = get_test_pool(Some(server_db_url)).await;
            let ws_svc = WsService::new(pool.clone());
            let user_svc = UserService::new(pool.clone(), ws_svc.clone());
            let chat_svc = ChatService::new(pool.clone(), user_svc.clone());
            let msg_svc = MsgService::new(pool.clone(), config.server.base_dir.clone());
            Ok((
                Self {
                    inner: Arc::new(AppStateInner {
                        config,
                        ek,
                        dk,
                        pool,
                        chat_svc,
                        user_svc,
                        ws_svc,
                        msg_svc,
                    }),
                },
                tdb,
            ))
        }
    }

    pub async fn get_test_pool(url: Option<&str>) -> (TestPg, PgPool) {
        let url = match url {
            Some(url) => url.to_owned(),
            None => "postgres://postgres:postgres@localhost:5432".to_owned(),
        };

        let tdb = TestPg::new(url, std::path::Path::new("../migrations"));
        let pool = tdb.get_pool().await;

        let sqls = include_str!("../fixtures/test.sql").split(';');
        let mut ts = pool.begin().await.expect("begin transaction failed");
        for sql in sqls {
            if sql.trim().is_empty() {
                continue;
            }
            ts.execute(sql).await.expect("execute sql failed");
        }
        ts.commit().await.expect("commit transaction failed");
        (tdb, pool)
    }

    #[allow(dead_code)]
    pub async fn get_test_state_and_pg() -> Result<(AppState, TestPg)> {
        let config: AppConfig = AppConfig::try_load()?;
        Ok(AppState::try_test_new(config).await?)
    }

    pub async fn get_test_state_and_pg_from_config_reader<T: std::io::Read>(
        reader: T,
    ) -> Result<(AppState, TestPg)> {
        let config = AppConfig::try_load_from_reader(reader)?;
        Ok(AppState::try_test_new(config).await?)
    }
}
