use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

use crate::error::AppError;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, PartialEq)]
pub struct Workspace {
    pub id: i64,
    pub name: String,
    pub owner_id: i64,
    pub created_at: DateTime<Utc>,
}

impl Workspace {
    pub async fn update_owner(&self, owner_id: u64, pool: &PgPool) -> Result<Workspace, AppError> {
        // update owner_id in two cases 1) owner_id = 0 2) owner's ws_id = id
        let ws = sqlx::query_as(
            r#"
        UPDATE workspaces
        SET owner_id = $1
        WHERE id = $2 and (SELECT ws_id FROM users WHERE id = $1) = $2
        RETURNING id, name, owner_id, created_at
        "#,
        )
        .bind(owner_id as i64)
        .bind(self.id)
        .fetch_one(pool)
        .await?;

        Ok(ws)
    }
}
