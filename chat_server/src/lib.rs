use std::{fmt, ops::Deref, sync::Arc, time::Duration};

use anyhow::Context;
use axum::{
    middleware::from_fn_with_state,
    routing::{get, post},
    Router,
};
use config::{AppConfig, AuthConfig};
use error::AppError;
use handlers::{
    create_chat_handler, delete_chat_handler, get_chat_handler, index_handler, list_chat_handler,
    list_chat_users_handler, list_message_handler, send_message_handler, signin_handler,
    signup_handler, update_chat_handler,
};

pub mod config;
mod error;
mod handlers;
mod middlewares;
mod models;
mod utils;

use middlewares::{set_layer, verify_token};
pub use models::User;
use sqlx::{postgres::PgPoolOptions, PgPool};
use utils::{DecodingKey, EncodingKey};
#[derive(Debug, Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

#[allow(unused)]
pub struct AppStateInner {
    pub(crate) config: AppConfig,
    pub(crate) ek: EncodingKey,
    pub(crate) dk: DecodingKey,
    pub(crate) pool: PgPool,
}
pub async fn get_router(config: AppConfig) -> Result<Router, AppError> {
    let state = AppState::try_new(config).await?;

    let api = Router::new()
        .route("/users", get(list_chat_users_handler))
        .route("/chats", get(list_chat_handler).post(create_chat_handler))
        .route(
            "/chats/:id",
            get(get_chat_handler)
                .patch(update_chat_handler)
                .delete(delete_chat_handler)
                .post(send_message_handler),
        )
        .route("/chats/:id/message", get(list_message_handler))
        .layer(from_fn_with_state(state.clone(), verify_token))
        .route("/signin", post(signin_handler))
        .route("/signup", post(signup_handler));

    let app = Router::new()
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
        let (ek, dk) = Self::load_key(&config.auth)?;
        let pool = PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(1000))
            .connect(&config.server.db_url)
            .await
            .context("connect db failed")?;
        Ok(Self {
            inner: Arc::new(AppStateInner {
                config,
                ek,
                dk,
                pool,
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

#[cfg(test)]
mod test_util {
    use std::sync::Arc;

    use anyhow::Result;
    use sqlx::Executor;
    use sqlx::PgPool;
    use sqlx_db_tester::TestPg;

    use crate::{config::AppConfig, error::AppError, AppState, AppStateInner};

    impl AppState {
        pub async fn try_test_new(
            config: AppConfig,
        ) -> Result<(Self, sqlx_db_tester::TestPg), AppError> {
            let (ek, dk) = Self::load_key(&config.auth)?;
            // let server_db_url = config.server.db_url.rsplitn(2, '/').skip(1).next().unwrap();
            let (server_db_url, _) = config.server.db_url.rsplit_once('/').unwrap();
            let (tdb, pool) = get_test_pool(Some(server_db_url)).await;
            Ok((
                Self {
                    inner: Arc::new(AppStateInner {
                        config,
                        ek,
                        dk,
                        pool,
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

    pub async fn get_test_state_and_pg() -> Result<(AppState, TestPg)> {
        let config = AppConfig::load()?;
        Ok(AppState::try_test_new(config).await?)
    }
}
