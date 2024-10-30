use axum_login::AuthUser;
use serde::{Deserialize, Serialize};
use sqlx::{Result, SqlitePool};
use time::OffsetDateTime;

use super::Model;

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
            .field("created_at", &self.created_at)
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
pub struct Default {
    user_id: i64
}

impl Default {
    #[allow(clippy::unused_self)]
    pub fn created_at(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

impl Model for User {
    type Default = Default;
    const DEFAULT: Default = Default {
        user_id: -1
    };

    async fn create(&mut self, pool: &SqlitePool) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO users (username, password_hash, created_at)
            VALUES (?, ?, ?)
            RETURNING user_id
            "#,
            self.username,
            self.password_hash,
            self.created_at
        )
        .fetch_one(pool)
        .await?;

        self.user_id = result.user_id;
        
        Ok(self.user_id)
    }

    async fn get_using_id(pool: &SqlitePool, id: i64) -> Result<Self> {
        sqlx::query_as!(
            User,
            r#"
            SELECT *
            FROM users
            WHERE user_id = ?
            "#,
            id
        )
        .fetch_one(pool)
        .await
    }

    async fn update(&self, pool: &SqlitePool) -> Result<bool> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE users
            SET username = ?, password_hash = ?, created_at = ?
            WHERE user_id = ?
            "#,
            self.username,
            self.password_hash,
            self.created_at,
            self.user_id
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }

    async fn delete_using_id(pool: &SqlitePool, id: i64) -> Result<bool> {
        let rows_affected = sqlx::query!(
            r#"
            DELETE
            FROM users
            WHERE user_id = ?
            "#,
            id
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }
}

impl User {
    pub async fn get_using_username(pool: &SqlitePool, username: &str) -> Result<Self> {
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

    #[must_use] pub fn to_redacted_clone(&self) -> Self {
        Self {
            user_id: self.user_id,
            username: self.username.clone(),
            password_hash: "[redacted]".to_string(),
            created_at: self.created_at
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn create(pool: SqlitePool) -> Result<()> {
        let mut user = User {
            user_id: User::DEFAULT.user_id,
            username: "test_user".to_string(),
            password_hash: "test_hash".to_string(),
            created_at: User::DEFAULT.created_at(),
        };

        user.create(&pool).await?;
        
        assert_eq!(user.user_id, 4);

        let returned_user = User::get_using_id(&pool, 4).await?;

        assert_eq!(returned_user.username, user.username);
        assert_eq!(returned_user.password_hash, user.password_hash);
        assert_eq!(returned_user.created_at, user.created_at);
        
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn create_existing(pool: SqlitePool) -> Result<()> {
        let mut user = User {
            user_id: User::DEFAULT.user_id,
            username: "piotrpdev".to_string(),
            password_hash: "test_hash".to_string(),
            created_at: User::DEFAULT.created_at(),
        };

        let returned_user_result = user.create(&pool).await;

        assert!(returned_user_result.is_err());

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn get(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
        let user_id = 2;
        let returned_user = User::get_using_id(&pool, user_id).await?;
        
        assert_eq!(returned_user.user_id, user_id);
        assert_eq!(returned_user.username, "piotrpdev");
        assert_eq!(returned_user.password_hash, "$argon2id$v=19$m=19456,t=2,p=1$VE0e3g7DalWHgDwou3nuRA$uC6TER156UQpk0lNQ5+jHM0l5poVjPA1he/Tyn9J4Zw");
        assert_eq!(returned_user.created_at, OffsetDateTime::from_unix_timestamp(1_729_530_138)?);
        
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn get_using_username(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
        let username = "piotrpdev";

        let returned_user = User::get_using_username(&pool, username).await?;
        
        assert_eq!(returned_user.user_id, 2);
        assert_eq!(returned_user.username, username);
        assert_eq!(returned_user.password_hash, "$argon2id$v=19$m=19456,t=2,p=1$VE0e3g7DalWHgDwou3nuRA$uC6TER156UQpk0lNQ5+jHM0l5poVjPA1he/Tyn9J4Zw");
        assert_eq!(returned_user.created_at, OffsetDateTime::from_unix_timestamp(1_729_530_138)?);
        
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn update(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
        let old_user = User::get_using_id(&pool, 2).await?;

        let updated_user = User {
            user_id: old_user.user_id,
            username: "new_joedaly".to_string(),
            password_hash: old_user.password_hash,
            created_at: OffsetDateTime::from_unix_timestamp(1_729_530_138)?,
        };

        let updated = updated_user.update(&pool).await?;
        
        assert!(updated);

        let returned_user = User::get_using_id(&pool, old_user.user_id).await?;

        assert_eq!(returned_user.user_id, updated_user.user_id);
        assert_eq!(returned_user.username, updated_user.username);
        assert_eq!(returned_user.password_hash, updated_user.password_hash);
        assert_eq!(returned_user.created_at, updated_user.created_at);
        
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn delete(pool: SqlitePool) -> Result<()> {
        let user_id = 2;
        let deleted = User::delete_using_id(&pool, user_id).await?;
        
        assert!(deleted);

        let returned_user = User::get_using_id(&pool, user_id).await;

        assert!(returned_user.is_err());
        
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn to_redacted_clone(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
        let user_id = 2;

        let returned_user = User::get_using_id(&pool, user_id).await?;
        let redacted_user = returned_user.to_redacted_clone();
        
        assert_eq!(redacted_user.user_id, user_id);
        assert_eq!(redacted_user.username, "piotrpdev");
        assert_eq!(redacted_user.password_hash, "[redacted]");
        assert_eq!(redacted_user.created_at, OffsetDateTime::from_unix_timestamp(1_729_530_138)?);
        
        Ok(())
    }
}