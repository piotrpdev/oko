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

pub struct UserDefaults {}

impl UserDefaults {
    pub fn created_at(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

impl User {
    pub const DEFAULT: UserDefaults = UserDefaults {};

    pub async fn create(pool: &SqlitePool, username: &str, password_hash: &str, created_at: OffsetDateTime) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO users (username, password_hash, created_at)
            VALUES (?, ?, ?)
            RETURNING user_id
            "#,
            username,
            password_hash,
            created_at
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

    pub async fn update(pool: &SqlitePool, user_id: i64, username: &str, password_hash: &str, created_at: OffsetDateTime) -> Result<bool> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE users
            SET username = ?, password_hash = ?, created_at = ?
            WHERE user_id = ?
            "#,
            username,
            password_hash,
            created_at,
            user_id
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

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn create(pool: SqlitePool) -> Result<()> {
        let username = "test_user";
        let password_hash = "test_hash";
        let created_at = User::DEFAULT.created_at();

        let test_user_id = User::create(&pool, username, password_hash, created_at).await?;
        
        assert_eq!(test_user_id, 4);

        let test_user = User::get(&pool, 4).await?;

        assert_eq!(test_user.username, username);
        assert_eq!(test_user.password_hash, password_hash);
        assert_eq!(test_user.created_at, created_at);
        
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn create_existing(pool: SqlitePool) -> Result<()> {
        let username = "piotrpdev";
        let password_hash = "test_hash";
        let created_at = User::DEFAULT.created_at();

        let test_user_id = User::create(&pool, username, password_hash, created_at).await;

        assert!(test_user_id.is_err());

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn get(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
        let user_id = 2;
        let test_user = User::get(&pool, user_id).await?;
        
        assert_eq!(test_user.user_id, user_id);
        assert_eq!(test_user.username, "piotrpdev");
        assert_eq!(test_user.password_hash, "$argon2id$v=19$m=19456,t=2,p=1$VE0e3g7DalWHgDwou3nuRA$uC6TER156UQpk0lNQ5+jHM0l5poVjPA1he/Tyn9J4Zw");
        assert_eq!(test_user.created_at, OffsetDateTime::from_unix_timestamp(1729530138)?);
        
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn get_using_username(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
        let username = "piotrpdev";

        let test_user = User::get_using_username(&pool, username).await?;
        
        assert_eq!(test_user.user_id, 2);
        assert_eq!(test_user.username, username);
        assert_eq!(test_user.password_hash, "$argon2id$v=19$m=19456,t=2,p=1$VE0e3g7DalWHgDwou3nuRA$uC6TER156UQpk0lNQ5+jHM0l5poVjPA1he/Tyn9J4Zw");
        assert_eq!(test_user.created_at, OffsetDateTime::from_unix_timestamp(1729530138)?);
        
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn update(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
        let user_id = 2;
        let username = "new_joedaly";
        let password_hash = "new_hash";
        let created_at = OffsetDateTime::from_unix_timestamp(1729530138)?;

        let updated = User::update(&pool, user_id, username, password_hash, created_at).await?;
        
        assert!(updated);

        let test_user = User::get(&pool, user_id).await?;

        assert_eq!(test_user.user_id, user_id);
        assert_eq!(test_user.username, username);
        assert_eq!(test_user.password_hash, password_hash);
        assert_eq!(test_user.created_at, created_at);
        
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn delete(pool: SqlitePool) -> Result<()> {
        let user_id = 2;
        let deleted = User::delete(&pool, user_id).await?;
        
        assert!(deleted);

        let test_user = User::get(&pool, user_id).await;

        assert!(test_user.is_err());
        
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn to_redacted_clone(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
        let user_id = 2;

        let test_user = User::get(&pool, user_id).await?;
        let redacted_user = test_user.to_redacted_clone();
        
        assert_eq!(redacted_user.user_id, user_id);
        assert_eq!(redacted_user.username, "piotrpdev");
        assert_eq!(redacted_user.password_hash, "[redacted]");
        assert_eq!(redacted_user.created_at, OffsetDateTime::from_unix_timestamp(1729530138)?);
        
        Ok(())
    }
}