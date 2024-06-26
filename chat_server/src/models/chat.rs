use crate::AppError;

use super::{Chat, ChatType, ChatUser};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateChat {
    pub name: Option<String>,
    pub members: Vec<i64>,
    pub public: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateChat {
    pub name: Option<String>,
}

impl Chat {
    pub async fn create(input: CreateChat, ws_id: u64, pool: &PgPool) -> Result<Self, AppError> {
        let len = match input.members.len() {
            len if len < 2 => {
                return Err(AppError::CreateChatError(
                    "Chat must have at least 2 members".to_string(),
                ))
            }
            len if len > 8 && input.name.is_none() => {
                return Err(AppError::CreateChatError(
                    "Group chat with more than 8 members must have a name".to_string(),
                ))
            }
            len => len,
        };

        let users = ChatUser::fetch_by_ids(&input.members, pool).await?;
        if users.len() != len {
            return Err(AppError::CreateChatError(
                "Some members do not exist".to_string(),
            ));
        }

        let chat_type = match (&input.name, len) {
            (None, 2) => ChatType::Single,
            (None, _) => ChatType::Group,
            (Some(_), _) => {
                if input.public {
                    ChatType::PublicChannel
                } else {
                    ChatType::PrivateChannel
                }
            }
        };

        let chat = sqlx::query_as(
            r#"
            INSERT INTO chats (ws_id, name, type, members)
            VALUES ($1, $2, $3, $4)
            RETURNING id, ws_id, name, type, members, created_at
            "#,
        )
        .bind(ws_id as i64)
        .bind(input.name)
        .bind(chat_type)
        .bind(input.members)
        .fetch_one(pool)
        .await?;

        Ok(chat)
    }

    pub async fn delete(ws_id: u64, chat_id: u64, pool: &PgPool) -> Result<Self, AppError> {
        if let Some(chat) = Self::get_by_id(chat_id, pool).await? {
            if chat.ws_id as u64 != ws_id {
                return Err(AppError::PermissionDeny);
            }
            let chat = sqlx::query_as(
                r#"
                DELETE FROM chats
                WHERE id = $1
                RETURNING id, ws_id, name, type, members, created_at
                "#,
            )
            .bind(chat_id as i64)
            .fetch_one(pool)
            .await?;
            Ok(chat)
        } else {
            Err(AppError::NotFound("chat id not found".to_owned()))
        }
    }

    pub async fn update(
        input: UpdateChat,
        ws_id: u64,
        chat_id: u64,
        pool: &PgPool,
    ) -> Result<Self, AppError> {
        if let Some(chat) = Self::get_by_id(chat_id, pool).await? {
            if chat.ws_id as u64 != ws_id {
                return Err(AppError::PermissionDeny);
            }
            let chat = sqlx::query_as(
                r#"
                update chats
                SET name = $1
                WHERE id = $2
                RETURNING id, ws_id, name, type, members, created_at
                "#,
            )
            .bind(input.name)
            .bind(chat_id as i64)
            .fetch_one(pool)
            .await?;
            Ok(chat)
        } else {
            Err(AppError::NotFound("chat id not found".to_owned()))
        }
    }

    pub async fn fetch_all(ws_id: u64, pool: &PgPool) -> Result<Vec<Self>, AppError> {
        let chats = sqlx::query_as(
            r#"
            SELECT id, ws_id, name, type, members, created_at
            FROM chats
            WHERE ws_id = $1
            "#,
        )
        .bind(ws_id as i64)
        .fetch_all(pool)
        .await?;

        Ok(chats)
    }

    pub async fn get_by_id(id: u64, pool: &PgPool) -> Result<Option<Self>, AppError> {
        let chat = sqlx::query_as(
            r#"
            SELECT id, ws_id, name, type, members, created_at
            FROM chats
            WHERE id = $1
            "#,
        )
        .bind(id as i64)
        .fetch_optional(pool)
        .await?;

        Ok(chat)
    }
}

#[cfg(test)]
impl CreateChat {
    pub fn new(name: Option<String>, members: &[i64], public: bool) -> Self {
        Self {
            name,
            members: members.to_vec(),
            public,
        }
    }
}

#[cfg(test)]
impl UpdateChat {
    pub fn new(name: Option<String>) -> Self {
        Self { name }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_util::get_test_pool;

    use super::*;

    #[tokio::test]
    async fn create_single_chat_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;
        let input = CreateChat::new(None, &[1, 2], false);
        let chat = Chat::create(input, 1, &pool)
            .await
            .expect("create chat failed");
        assert_eq!(chat.ws_id, 1);
        assert_eq!(chat.members.len(), 2);
        assert_eq!(chat.r#type, ChatType::Single);
    }

    #[tokio::test]
    async fn create_public_name_chat_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;
        let input = CreateChat::new(Some("test".to_string()), &[1, 2, 3], true);
        let chat = Chat::create(input, 1, &pool)
            .await
            .expect("create chat failed");
        assert_eq!(chat.ws_id, 1);
        assert_eq!(chat.members.len(), 3);
        assert_eq!(chat.r#type, ChatType::PublicChannel);
        assert_eq!(chat.name, Some("test".to_string()));
    }

    #[tokio::test]
    pub async fn chat_get_by_id_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;
        let chat = Chat::get_by_id(1, &pool)
            .await
            .expect("get chat by id failed")
            .unwrap();
        assert_eq!(chat.members.len(), 5);
        assert_eq!(chat.name.unwrap(), "general");
        assert_eq!(chat.ws_id, 1);
    }
    #[tokio::test]
    pub async fn chat_get_all_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;
        let chats = Chat::fetch_all(1, &pool).await.expect("get all chat fail");
        assert_eq!(chats.len(), 4);
    }
    #[tokio::test]
    pub async fn chat_delete_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;
        let chat = Chat::delete(1, 1, &pool).await.expect("delete chat fail");
        assert_eq!(chat.name.unwrap(), "general");
        let chat = Chat::get_by_id(1, &pool)
            .await
            .expect("get chat by id failed");
        assert!(chat.is_none())
    }
    #[tokio::test]
    pub async fn chat_delete_other_ws_chat_should_fail() {
        let (_tdb, pool) = get_test_pool(None).await;
        match Chat::delete(2, 1, &pool).await {
            Err(AppError::PermissionDeny) => return,
            _ => panic!("should fail"),
        };
    }

    #[tokio::test]
    pub async fn chat_update_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;
        let input = UpdateChat::new(Some("test".to_string()));
        Chat::update(input, 1, 1, &pool)
            .await
            .expect("update chat fail");
        let chat = Chat::get_by_id(1, &pool)
            .await
            .expect("get chat by id failed")
            .unwrap();
        assert_eq!(chat.name.unwrap(), "test");
    }
}
