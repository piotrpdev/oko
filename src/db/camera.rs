use serde::{Deserialize, Serialize};
use sqlx::{Result, SqlitePool};
use time::OffsetDateTime;

use crate::db::CameraPermissionView;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera {
    pub camera_id: i64,
    pub name: String,
    pub ip_address: Option<String>,
    pub last_connected: Option<OffsetDateTime>,
    pub is_active: bool,
}

#[allow(dead_code)]
impl Camera {
    pub async fn create(
        pool: &SqlitePool,
        name: &str,
        ip_address: Option<&str>,
    ) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO cameras (name, ip_address)
            VALUES (?, ?)
            RETURNING camera_id
            "#,
            name,
            ip_address
        )
        .fetch_one(pool)
        .await?;

        Ok(result.camera_id)
    }

    pub async fn get(pool: &SqlitePool, camera_id: i64) -> Result<Camera> {
        sqlx::query_as!(
            Camera,
            r#"
            SELECT *
            FROM cameras
            WHERE camera_id = ?
            "#,
            camera_id
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        camera_id: i64,
        name: &str,
        ip_address: Option<&str>,
        is_active: bool,
    ) -> Result<bool> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE cameras
            SET name = ?, ip_address = ?, is_active = ?
            WHERE camera_id = ?
            "#,
            name,
            ip_address,
            is_active,
            camera_id
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }

    pub async fn delete(pool: &SqlitePool, camera_id: i64) -> Result<bool> {
        let rows_affected = sqlx::query!("DELETE FROM cameras WHERE camera_id = ?", camera_id)
            .execute(pool)
            .await?
            .rows_affected();

            Ok(rows_affected > 0)
    }

    pub async fn list_accessible_to_user(
        db: &SqlitePool,
        user_id: i64,
    ) -> Result<Vec<CameraPermissionView>> {
        sqlx::query_as!(
            CameraPermissionView,
            r#"
            SELECT c.camera_id, c.name as camera_name, cp.can_view, cp.can_control
            FROM cameras c
            JOIN camera_permissions cp ON c.camera_id = cp.camera_id
            WHERE cp.user_id = ? AND c.is_active = true
            "#,
            user_id
        )
        .fetch_all(db)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(fixtures("cameras"))]
    async fn create(pool: SqlitePool) -> sqlx::Result<()> {
        let camera_name = "Test Camera";
        let ip_address: std::option::Option<std::string::String> = None;
        
        let camera_id = Camera::create(&pool, camera_name, ip_address.as_deref()).await?;

        assert_eq!(camera_id, 3);

        let camera = Camera::get(&pool, camera_id).await?;

        assert_eq!(camera.name, camera_name);
        assert_eq!(camera.ip_address, ip_address);
        assert!(camera.is_active);

        Ok(())
    }

    #[sqlx::test(fixtures("cameras"))]
    async fn get(pool: SqlitePool) -> sqlx::Result<()> {
        let camera_id = 1;

        let camera = Camera::get(&pool, camera_id).await?;

        assert_eq!(camera.camera_id, camera_id);
        assert_eq!(camera.name, "Front Door");
        assert_eq!(camera.ip_address, Some("192.168.0.169".to_string()));
        assert!(camera.is_active);

        Ok(())
    }

    #[sqlx::test(fixtures("cameras"))]
    async fn update(pool: SqlitePool) -> sqlx::Result<()> {
        let camera_id = 1;
        let camera_name = "Updated Camera";
        let ip_address: std::option::Option<std::string::String> = Some("192.168.0.24".to_string());
        let is_active = false;

        let updated = Camera::update(&pool, camera_id, camera_name, ip_address.as_deref(), is_active).await?;

        assert!(updated);

        let camera = Camera::get(&pool, 1).await?;

        assert_eq!(camera.name, camera_name);
        assert_eq!(camera.ip_address, ip_address);
        assert!(!camera.is_active);

        Ok(())
    }

    #[sqlx::test(fixtures("cameras"))]
    async fn delete(pool: SqlitePool) -> sqlx::Result<()> {
        let camera_id = 1;
        let deleted = Camera::delete(&pool, camera_id).await?;

        assert!(deleted);

        let camera = Camera::get(&pool, camera_id).await;

        assert!(camera.is_err());

        Ok(())
    }

    #[sqlx::test(fixtures("users", "cameras", "camera_permissions"))]
    async fn list_accessible_to_user(pool: SqlitePool) -> sqlx::Result<()> {
        let cameras = Camera::list_accessible_to_user(&pool, 3).await?;

        assert_eq!(cameras.len(), 2);

        assert_eq!(cameras[0].camera_id, 1);
        assert_eq!(cameras[0].camera_name, "Front Door");
        assert!(cameras[0].can_view);
        assert!(!cameras[0].can_control);

        assert_eq!(cameras[1].camera_id, 2);
        assert_eq!(cameras[1].camera_name, "Kitchen");
        assert!(!cameras[1].can_view);
        assert!(!cameras[1].can_control);

        Ok(())
    }
}