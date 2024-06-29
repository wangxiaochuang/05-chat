use std::{mem, sync::Arc};

use crate::{
    error::AppError,
    models::{ChatUser, CreateUser, SigninUser, User},
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2, PasswordHash, PasswordVerifier,
};
use sqlx::PgPool;

use super::WsService;

pub(crate) struct UserService {
    pool: PgPool,
    ws_svc: Arc<WsService>,
}

impl Clone for UserService {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            ws_svc: self.ws_svc.clone(),
        }
    }
}

impl UserService {
    pub fn new(pool: PgPool, ws_svc: WsService) -> Self {
        Self {
            pool,
            ws_svc: Arc::new(ws_svc),
        }
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as(
            "select id, ws_id, fullname, email, password_hash, created_at from users where email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn create(&self, input: &CreateUser) -> Result<User, AppError> {
        let user = self.find_by_email(&input.email).await?;
        if user.is_some() {
            return Err(AppError::EmailAlreadyExists(input.email.to_string()));
        }
        let ws = match self.ws_svc.find_by_name(&input.workspace).await? {
            Some(ws) => ws,
            None => self.ws_svc.create(&input.workspace, 0).await?,
        };
        let password_hash = hash_password(&input.password)?;
        let user: User = sqlx::query_as(
            r#"
        insert into users (ws_id, email, fullname, password_hash)
        values ($1, $2, $3, $4)
        returning id, ws_id, fullname, email, created_at
        "#,
        )
        .bind(ws.id)
        .bind(&input.email)
        .bind(&input.fullname)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await?;

        if ws.owner_id == 0 {
            ws.update_owner(user.id as _, &self.pool).await?;
        }
        Ok(user)
    }

    /// Verify email and password
    pub async fn verify(&self, input: &SigninUser) -> Result<Option<User>, AppError> {
        let user: Option<User> = sqlx::query_as(
            "select id, ws_id, fullname, email, password_hash, created_at from users where email = $1",
        )
        .bind(&input.email)
        .fetch_optional(&self.pool)
        .await?;

        match user {
            Some(mut user) => {
                let password_hash = mem::take(&mut user.password_hash).unwrap_or_default();
                let is_valid = verify_password(&input.password, &password_hash)?;
                if is_valid {
                    Ok(Some(user))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    pub async fn fetch_by_ids(&self, ids: &[i64]) -> Result<Vec<ChatUser>, AppError> {
        let users = sqlx::query_as(
            r#"
        select id, fullname, email
        from users
        where id = ANY($1)
        "#,
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }

    #[allow(dead_code)]
    pub async fn fetch_all(&self, ws_id: u64) -> Result<Vec<ChatUser>, AppError> {
        let users = sqlx::query_as(
            r#"
        select id, fullname, email
        from users
        where ws_id = $1
        "#,
        )
        .bind(ws_id as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }
}

fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let hasher = Argon2::default();
    let password_hash = hasher
        .hash_password(password.as_bytes(), &salt)?
        .to_string();
    Ok(password_hash)
}

fn verify_password(password: &str, password_hash: &str) -> Result<bool, AppError> {
    let hasher = Argon2::default();
    let password_hash = PasswordHash::new(password_hash)?;

    let is_valid = hasher
        .verify_password(password.as_bytes(), &password_hash)
        .is_ok();
    Ok(is_valid)
}

#[cfg(test)]
impl User {
    pub fn new(id: i64, fullname: &str, email: &str) -> Self {
        Self {
            id,
            ws_id: 0,
            fullname: fullname.to_string(),
            email: email.to_string(),
            password_hash: None,
            created_at: chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
impl CreateUser {
    pub fn new(ws: &str, fullname: &str, email: &str, password: &str) -> Self {
        Self {
            fullname: fullname.to_string(),
            workspace: ws.to_owned(),
            email: email.to_string(),
            password: password.to_string(),
        }
    }
}

#[cfg(test)]
impl SigninUser {
    pub fn new(email: &str, password: &str) -> Self {
        Self {
            email: email.to_string(),
            password: password.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_util::get_test_pool;

    use super::*;
    use anyhow::Result;

    #[test]
    fn hash_password_and_verify_should_work() -> Result<()> {
        let password = "123456";
        let password_hash = hash_password(password)?;
        assert_eq!(password_hash.len(), 97);
        assert!(verify_password(password, &password_hash)?);
        Ok(())
    }
    #[tokio::test]
    async fn create_duplicate_user_should_fail() -> Result<()> {
        let (_tdb, pool) = get_test_pool(None).await;
        let ws_svc = WsService::new(pool.clone());
        let svc = UserService::new(pool, ws_svc);
        let input = CreateUser::new("none", "jack1", "jack1@gmail.com", "123456");
        match svc.create(&input).await {
            Err(AppError::EmailAlreadyExists(email)) => {
                assert_eq!(email, "jack1@gmail.com");
            }
            _ => panic!("should return EmailAlreadyExists"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn create_and_verify_user_should_work() -> Result<()> {
        let (_tdb, pool) = get_test_pool(None).await;
        let ws_svc = WsService::new(pool.clone());
        let svc = UserService::new(pool, ws_svc);
        let input = CreateUser::new("none", "jack", "jack@admin", "123456");
        let user = svc.create(&input).await?;
        assert_eq!(user.email, input.email);
        assert_eq!(user.fullname, input.fullname);
        assert!(user.id > 0);

        let user = svc.find_by_email(&input.email).await?;
        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.email, input.email);
        assert_eq!(user.fullname, input.fullname);

        let input = SigninUser::new(&input.email, &input.password);
        let user = svc.verify(&input).await?;
        assert!(user.is_some());

        Ok(())
    }
}
