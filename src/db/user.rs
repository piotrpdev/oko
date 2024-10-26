use axum_login::AuthUser;
use serde::{Deserialize, Serialize};
use sqlx::{Result, SqlitePool};
use time::OffsetDateTime;

#[derive(Clone, Serialize, Deserialize)]
pub struct User {
    pub user_id: i64,
    pub username: String,
    pub password_hash: String,
    pub created_at: OffsetDateTime,
}

// Here we've implemented `Debug` manually to avoid accidentally logging the
// password hash.
impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("user_id", &self.user_id)
            .field("username", &self.username)
            .field("password_hash", &"[redacted]")
            .finish()
    }
}

impl AuthUser for User {
    type Id = i64;

    fn id(&self) -> Self::Id {
        self.user_id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.as_bytes() // We use the password hash as the auth
                                 // hash--what this means
                                 // is when the user changes their password the
                                 // auth session becomes invalid.
    }
}

#[allow(dead_code)]
impl User {
    pub async fn create(pool: &SqlitePool, username: &str, password_hash: &str) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO users (username, password_hash)
            VALUES (?, ?)
            RETURNING user_id
            "#,
            username,
            password_hash
        )
        .fetch_one(pool)
        .await?;
        
        Ok(result.user_id)
    }

    pub async fn get(pool: &SqlitePool, user_id: i64) -> Result<User> {
        sqlx::query_as!(
            User,
            r#"
            SELECT *
            FROM users
            WHERE user_id = ?
            "#,
            user_id
        )
        .fetch_one(pool)
        .await
    }

    pub async fn get_using_username(pool: &SqlitePool, username: &str) -> Result<User> {
        sqlx::query_as!(
            User,
            r#"
            SELECT *
            FROM users
            WHERE username = ?
            "#,
            username
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(pool: &SqlitePool, user_id: i64, username: &str, password_hash: &str) -> Result<bool> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE users
            SET username = ?, password_hash = ?
            WHERE user_id = ?
            "#,
            username,
            user_id,
            password_hash
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }

    pub async fn delete(pool: &SqlitePool, user_id: i64) -> Result<bool> {
        let rows_affected = sqlx::query!(
            r#"
            DELETE
            FROM users
            WHERE user_id = ?
            "#,
            user_id
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }

    pub fn to_redacted_clone(&self) -> User {
        User {
            user_id: self.user_id,
            username: self.username.clone(),
            password_hash: "[redacted]".to_string(),
            created_at: self.created_at
        }
    }
}
