use std::sync::Arc;

use crate::AppError;

use chat_core::{Chat, ChatType};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;

use super::UserService;

#[derive(Debug, Clone, ToSchema, Default, Serialize, Deserialize)]
pub struct CreateChat {
    /// chat name
    pub name: Option<String>,
    /// chat members
    pub members: Vec<i64>,
    /// whether it is public
    pub public: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateChat {
    pub name: Option<String>,
}

pub struct ChatService {
    pool: PgPool,
    user_svc: Arc<UserService>,
}

impl Clone for ChatService {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            user_svc: self.user_svc.clone(),
        }
    }
}

impl ChatService {
    pub fn new(pool: PgPool, user_svc: UserService) -> Self {
        Self {
            pool,
            user_svc: Arc::new(user_svc),
        }
    }

    pub async fn create(&self, input: CreateChat, ws_id: u64) -> Result<Chat, AppError> {
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

        let users = self.user_svc.fetch_by_ids(&input.members).await?;
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
        .fetch_one(&self.pool)
        .await?;

        Ok(chat)
    }

    pub async fn update(
        &self,
        input: UpdateChat,
        ws_id: u64,
        chat_id: u64,
    ) -> Result<Chat, AppError> {
        if let Some(chat) = self.get_by_id(chat_id).await? {
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
            .fetch_one(&self.pool)
            .await?;
            Ok(chat)
        } else {
            Err(AppError::NotFound("chat id not found".to_owned()))
        }
    }
    pub async fn delete(&self, ws_id: u64, chat_id: u64) -> Result<Chat, AppError> {
        if let Some(chat) = self.get_by_id(chat_id).await? {
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
            .fetch_one(&self.pool)
            .await?;
            Ok(chat)
        } else {
            Err(AppError::NotFound("chat id not found".to_owned()))
        }
    }
    pub async fn get_by_id(&self, id: u64) -> Result<Option<Chat>, AppError> {
        let chat = sqlx::query_as(
            r#"
            SELECT id, ws_id, name, type, members, created_at
            FROM chats
            WHERE id = $1
            "#,
        )
        .bind(id as i64)
        .fetch_optional(&self.pool)
        .await?;

        Ok(chat)
    }

    pub async fn fetch_all(&self, ws_id: u64) -> Result<Vec<Chat>, AppError> {
        let chats = sqlx::query_as(
            r#"
            SELECT id, ws_id, name, type, members, created_at
            FROM chats
            WHERE ws_id = $1
            "#,
        )
        .bind(ws_id as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(chats)
    }

    pub async fn is_chat_member(&self, chat_id: u64, user_id: u64) -> Result<bool, AppError> {
        let is_member = sqlx::query(
            r#"
            SELECT 1
            FROM chats
            WHERE id = $1 AND $2 = ANY(members)
            "#,
        )
        .bind(chat_id as i64)
        .bind(user_id as i64)
        .fetch_optional(&self.pool)
        .await?;
        Ok(is_member.is_some())
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
    use crate::{services::WsService, test_util::get_test_pool};

    use super::*;

    #[tokio::test]
    async fn create_single_chat_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;
        let ws_svc = WsService::new(pool.clone());
        let user_svc = UserService::new(pool.clone(), ws_svc);
        let svc = ChatService::new(pool.clone(), user_svc);
        let input = CreateChat::new(None, &[1, 2], false);
        let chat = svc.create(input, 1).await.expect("create chat failed");
        assert_eq!(chat.ws_id, 1);
        assert_eq!(chat.members.len(), 2);
        assert_eq!(chat.r#type, ChatType::Single);
    }

    #[tokio::test]
    async fn create_public_name_chat_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;
        let ws_svc = WsService::new(pool.clone());
        let user_svc = UserService::new(pool.clone(), ws_svc);
        let svc = ChatService::new(pool.clone(), user_svc);
        let input = CreateChat::new(Some("test".to_string()), &[1, 2, 3], true);
        let chat = svc.create(input, 1).await.expect("create chat failed");
        assert_eq!(chat.ws_id, 1);
        assert_eq!(chat.members.len(), 3);
        assert_eq!(chat.r#type, ChatType::PublicChannel);
        assert_eq!(chat.name, Some("test".to_string()));
    }

    #[tokio::test]
    pub async fn chat_get_by_id_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;
        let ws_svc = WsService::new(pool.clone());
        let user_svc = UserService::new(pool.clone(), ws_svc);
        let svc = ChatService::new(pool.clone(), user_svc);
        let chat = svc
            .get_by_id(1)
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
        let ws_svc = WsService::new(pool.clone());
        let user_svc = UserService::new(pool.clone(), ws_svc);
        let svc = ChatService::new(pool.clone(), user_svc);
        let chats = svc.fetch_all(1).await.expect("get all chat fail");
        assert_eq!(chats.len(), 4);
    }
    #[tokio::test]
    pub async fn chat_delete_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;
        let ws_svc = WsService::new(pool.clone());
        let user_svc = UserService::new(pool.clone(), ws_svc);
        let svc = ChatService::new(pool.clone(), user_svc);
        let chat = svc.delete(1, 1).await.expect("delete chat fail");
        assert_eq!(chat.name.unwrap(), "general");
        let chat = svc.get_by_id(1).await.expect("get chat by id failed");
        assert!(chat.is_none())
    }
    #[tokio::test]
    pub async fn chat_delete_other_ws_chat_should_fail() {
        let (_tdb, pool) = get_test_pool(None).await;
        let ws_svc = WsService::new(pool.clone());
        let user_svc = UserService::new(pool.clone(), ws_svc);
        let svc = ChatService::new(pool.clone(), user_svc);
        match svc.delete(2, 1).await {
            Err(AppError::PermissionDeny) => return,
            _ => panic!("should fail"),
        };
    }

    #[tokio::test]
    pub async fn chat_update_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;
        let ws_svc = WsService::new(pool.clone());
        let user_svc = UserService::new(pool.clone(), ws_svc);
        let svc = ChatService::new(pool.clone(), user_svc);
        let input = UpdateChat::new(Some("test".to_string()));
        svc.update(input, 1, 1).await.expect("update chat fail");
        let chat = svc
            .get_by_id(1)
            .await
            .expect("get chat by id failed")
            .unwrap();
        assert_eq!(chat.name.unwrap(), "test");
    }

    #[tokio::test]
    pub async fn chat_is_member_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;
        let ws_svc = WsService::new(pool.clone());
        let user_svc = UserService::new(pool.clone(), ws_svc);
        let svc = ChatService::new(pool.clone(), user_svc);
        let is_member = svc
            .is_chat_member(1, 1)
            .await
            .expect("is chat member should work");
        assert!(is_member);

        let is_member = svc
            .is_chat_member(1, 6)
            .await
            .expect("is chat member should work");
        assert!(!is_member);
    }
}
