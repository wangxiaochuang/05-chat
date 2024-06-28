use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{error::AppError, models::ChatFile, AppState};

use super::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessage {
    pub content: String,
    pub files: Vec<String>,
}

impl AppState {
    pub async fn create_message(
        &self,
        input: CreateMessage,
        chat_id: u64,
        user_id: u64,
    ) -> Result<Message, AppError> {
        let base_dir = &self.config.server.base_dir;
        if input.content.is_empty() {
            return Err(AppError::InvalidInput("content is empty".to_string()));
        }

        for url in &input.files {
            let file = ChatFile::from_str(url)?;
            if !file.path(base_dir).exists() {
                return Err(AppError::InvalidInput("file not found".to_string()));
            }
        }

        let message = sqlx::query_as(
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
        .await?;

        Ok(message)
    }
}