use sqlx::PgPool;

use crate::{
    error::AppError,
    models::{ChatUser, Workspace},
};

pub(crate) struct WsService {
    pool: PgPool,
}

impl Clone for WsService {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}

impl WsService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, name: &str, user_id: u64) -> Result<Workspace, AppError> {
        let ws = sqlx::query_as(
            r#"
        INSERT INTO workspaces (name, owner_id)
        VALUES ($1, $2)
        RETURNING id, name, owner_id, created_at
        "#,
        )
        .bind(name)
        .bind(user_id as i64)
        .fetch_one(&self.pool)
        .await?;

        Ok(ws)
    }

    pub async fn find_by_name(&self, name: &str) -> Result<Option<Workspace>, AppError> {
        let ws = sqlx::query_as(
            r#"
        SELECT id, name, owner_id, created_at
        FROM workspaces
        WHERE name = $1
        "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(ws)
    }

    #[allow(dead_code)]
    pub async fn find_by_id(&self, id: u64) -> Result<Option<Workspace>, AppError> {
        let ws = sqlx::query_as(
            r#"
        SELECT id, name, owner_id, created_at
        FROM workspaces
        WHERE id = $1
        "#,
        )
        .bind(id as i64)
        .fetch_optional(&self.pool)
        .await?;

        Ok(ws)
    }

    #[allow(dead_code)]
    pub async fn fetch_all_chat_users(&self, id: u64) -> Result<Vec<ChatUser>, AppError> {
        let users = sqlx::query_as(
            r#"
        SELECT id, fullname, email
        FROM users
        WHERE ws_id = $1 order by id
        "#,
        )
        .bind(id as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::{models::CreateUser, services::UserService, test_util::get_test_pool};

    use super::*;

    #[tokio::test]
    async fn workspace_should_create_and_set_owner() {
        let (_tdb, pool) = get_test_pool(None).await;
        let svc = WsService::new(pool.clone());
        let user_svc = UserService::new(pool.clone(), svc.clone());

        let ws = svc.create("test", 0).await.unwrap();

        let input = CreateUser::new(&ws.name, "jack", "jack@gmail.org", "Hunter42");
        let user = user_svc.create(&input).await.unwrap();

        assert_eq!(ws.name, "test");

        assert_eq!(user.ws_id, ws.id);

        let ws = ws.update_owner(user.id as _, &pool).await.unwrap();

        assert_eq!(ws.owner_id, user.id);
    }

    #[tokio::test]
    async fn workspace_should_find_by_name() -> Result<()> {
        let (_tdb, pool) = get_test_pool(None).await;
        let svc = WsService::new(pool);
        let ws = svc.find_by_name("ws1").await?;
        assert_eq!(ws.unwrap().name, "ws1");

        Ok(())
    }

    #[tokio::test]
    async fn workspace_should_fetch_all_chat_users() -> Result<()> {
        let (_tdb, pool) = get_test_pool(None).await;
        let svc = WsService::new(pool);

        let users = svc.fetch_all_chat_users(1).await?;
        assert_eq!(users.len(), 5);
        assert_eq!(users[0].id, 1);
        assert_eq!(users[1].id, 2);
        assert_eq!(users[2].id, 3);
        assert_eq!(users[3].id, 4);
        assert_eq!(users[4].id, 5);
        Ok(())
    }
}
