use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use sqlx::PgPool;

use crate::{
    error::AppError,
    models::{ChatFile, CreateMessage, ListMessageOption, Message},
};

pub struct MsgService {
    pool: PgPool,
    base_dir: PathBuf,
}

impl MsgService {
    pub fn new(pool: PgPool, base_dir: impl AsRef<Path>) -> Self {
        Self {
            pool,
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    pub async fn create(
        &self,
        input: CreateMessage,
        chat_id: u64,
        user_id: u64,
    ) -> Result<Message, AppError> {
        if input.content.is_empty() {
            return Err(AppError::InvalidInput("content is empty".to_string()));
        }

        for url in &input.files {
            let file = ChatFile::from_str(url)?;
            if !file.path(&self.base_dir).exists() {
                return Err(AppError::InvalidInput("file not found".to_string()));
            }
        }

        Ok(sqlx::query_as(
            r#"
            INSERT INTO messages (chat_id, sender_id, content, files)
            VALUES ($1, $2, $3, $4)
            RETURNING id, chat_id, sender_id, content, files, created_at
            "#,
        )
        .bind(chat_id as i64)
        .bind(user_id as i64)
        .bind(input.content)
        .bind(input.files)
        .fetch_one(&self.pool)
        .await?)
    }
    pub async fn list(
        &self,
        input: ListMessageOption,
        chat_id: u64,
    ) -> Result<Vec<Message>, AppError> {
        let last_id = input.last_id.unwrap_or(i64::MAX as _);
        let messages = sqlx::query_as(
            r#"
        SELECT id, chat_id, sender_id, content, files, created_at
        FROM messages
        WHERE chat_id = $1
        AND id < $2
        ORDER BY id DESC
        LIMIT $3
        "#,
        )
        .bind(chat_id as i64)
        .bind(last_id as i64)
        .bind(input.limit as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(messages)
    }
}

#[cfg(test)]
impl CreateMessage {
    pub fn new(content: String, files: Vec<String>) -> Self {
        Self { content, files }
    }
}

#[cfg(test)]
impl ListMessageOption {
    pub fn new(last_id: Option<u64>, limit: u64) -> Self {
        Self { last_id, limit }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::test_util::get_test_pool;
    use anyhow::Result;
    use tempfile::tempdir;

    #[tokio::test]
    async fn create_message_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;
        let basedir = tempdir().expect("create tempfile");
        let svc = MsgService::new(pool, &basedir);
        let url = upload_dummy_file(&basedir).expect("upload dummy file should work");
        let input = CreateMessage::new("hello world".to_string(), vec![url.to_owned()]);
        let message = svc.create(input, 1, 1).await.expect("create message fail");
        assert_eq!(message.content, "hello world");
        assert_eq!(message.files, vec![url]);
    }

    #[tokio::test]
    async fn create_message_with_invalid_file_should_fail() {
        let (_tdb, pool) = get_test_pool(None).await;
        let basedir = tempdir().expect("create tempfile");
        let svc = MsgService::new(pool, basedir.into_path());
        let input = CreateMessage::new(
            "hello world".to_string(),
            vec!["invalid_file.txt".to_owned()],
        );
        let err = svc.create(input, 1, 1).await.unwrap_err();
        assert_eq!(err.to_string(), "invalid input: file path");
    }

    #[tokio::test]
    async fn list_message_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;
        let basedir = tempdir().expect("create tempfile");
        let svc = MsgService::new(pool, basedir.into_path());

        let input = ListMessageOption::new(None, 6);
        let messages = svc.list(input, 1).await.expect("list fail");
        assert_eq!(messages.len(), 6);

        let last_id = messages.last().unwrap().id as _;

        let input = ListMessageOption::new(Some(last_id), 6);
        let messages = svc.list(input, 1).await.expect("list fail");
        assert_eq!(messages.len(), 4);
    }

    fn upload_dummy_file(base_dir: impl AsRef<Path>) -> Result<String> {
        let content = b"hello world";
        let chat_file = ChatFile::new(1, "dummy.txt", content);
        let file_path = chat_file.path(base_dir);
        std::fs::create_dir_all(file_path.parent().expect("file path parent should exists"))
            .unwrap();
        std::fs::write(file_path, content).expect("write content should work");
        Ok(chat_file.url())
    }
}
