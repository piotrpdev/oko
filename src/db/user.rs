use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// TODO: Sync this struct with the one in users.rs
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub user_id: i64,
    pub username: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}