use serde::{Deserialize, Serialize};
use sqlx::{Result, SqlitePool};
use time::OffsetDateTime;

use crate::db::CameraPermissionView;

use super::Model;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera {
    pub camera_id: i64,
    pub name: String,
    pub ip_address: Option<String>,
    pub last_connected: Option<OffsetDateTime>,
    pub is_active: bool,
}

pub struct Default {
    pub camera_id: i64,
    pub ip_address: Option<String>,
    pub last_connected: Option<OffsetDateTime>,
    pub is_active: bool,
}

impl Model for Camera {
    type Default = Default;
    const DEFAULT: Default = Default {
        camera_id: -1,
        ip_address: None,
        last_connected: None,
        is_active: true,
    };

    async fn create_using_self(&mut self, pool: &SqlitePool) -> Result<()> {
        let result = sqlx::query!(
            r#"
            INSERT INTO cameras (name, ip_address, last_connected, is_active)
            VALUES (?, ?, ?, ?)
            RETURNING camera_id
            "#,
            self.name,
            self.ip_address,
            self.last_connected,
            self.is_active
        )
        .fetch_one(pool)
        .await?;

        self.camera_id = result.camera_id;

        Ok(())
    }

    async fn get_using_id(pool: &SqlitePool, camera_id: i64) -> Result<Self> {
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

    async fn update_using_self(&self, pool: &SqlitePool) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE cameras
            SET name = ?, ip_address = ?, last_connected = ?, is_active = ?
            WHERE camera_id = ?
            RETURNING camera_id
            "#,
            self.name,
            self.ip_address,
            self.last_connected,
            self.is_active,
            self.camera_id
        )
        .fetch_one(pool)
        .await?;

        Ok(())
    }

    async fn delete_using_id(pool: &SqlitePool, camera_id: i64) -> Result<()> {
        sqlx::query!(
            r#"
            DELETE
            FROM cameras
            WHERE camera_id = ?
            RETURNING camera_id
            "#,
            camera_id
        )
        .fetch_one(pool)
        .await?;

        Ok(())
    }
}

impl Camera {
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

    pub async fn get_using_ip(pool: &SqlitePool, ip_address: String) -> Result<Self> {
        sqlx::query_as!(
            Camera,
            r#"
            SELECT *
            FROM cameras
            WHERE ip_address = ?
            "#,
            ip_address
        )
        .fetch_one(pool)
        .await
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("cameras")))]
    async fn create(pool: SqlitePool) -> Result<()> {
        let mut camera = Camera {
            camera_id: Camera::DEFAULT.camera_id,
            name: "Test Camera".to_string(),
            ip_address: Camera::DEFAULT.ip_address,
            last_connected: Camera::DEFAULT.last_connected,
            is_active: Camera::DEFAULT.is_active,
        };

        camera.create_using_self(&pool).await?;

        assert_eq!(camera.camera_id, 3);

        let returned_camera = Camera::get_using_id(&pool, camera.camera_id).await?;

        assert_eq!(returned_camera.name, camera.name);
        assert_eq!(returned_camera.ip_address, camera.ip_address);
        assert_eq!(returned_camera.last_connected, camera.last_connected);
        assert!(returned_camera.is_active);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("cameras")))]
    async fn get(pool: SqlitePool) -> Result<()> {
        let camera_id = 1;

        let returned_camera = Camera::get_using_id(&pool, camera_id).await?;

        assert_eq!(returned_camera.camera_id, camera_id);
        assert_eq!(returned_camera.name, "Front Door");
        assert_eq!(
            returned_camera.ip_address,
            Some("127.0.0.1:40000".to_string())
        );
        assert!(returned_camera.is_active);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("cameras")))]
    async fn update(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
        let old_camera = Camera::get_using_id(&pool, 1).await?;

        let updated_camera = Camera {
            camera_id: old_camera.camera_id,
            name: old_camera.name,
            ip_address: Some("192.168.0.24".to_string()),
            last_connected: Some(time::OffsetDateTime::from_unix_timestamp(1_729_443_378)?),
            is_active: false,
        };

        let updated = updated_camera.update_using_self(&pool).await;

        assert!(updated.is_ok());

        let returned_camera = Camera::get_using_id(&pool, old_camera.camera_id).await?;

        assert_eq!(returned_camera.name, updated_camera.name);
        assert_eq!(returned_camera.ip_address, updated_camera.ip_address);
        assert_eq!(
            returned_camera.last_connected,
            updated_camera.last_connected
        );
        assert!(!returned_camera.is_active);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("cameras")))]
    async fn delete(pool: SqlitePool) -> Result<()> {
        let camera_id = 1;
        let deleted = Camera::delete_using_id(&pool, camera_id).await;

        assert!(deleted.is_ok());

        let returned_camera_result = Camera::get_using_id(&pool, camera_id).await;

        assert!(returned_camera_result.is_err());

        let impossible_deleted = Camera::delete_using_id(&pool, camera_id).await;

        assert!(impossible_deleted.is_err());

        Ok(())
    }

    #[sqlx::test(fixtures(
        path = "../../fixtures",
        scripts("users", "cameras", "camera_permissions")
    ))]
    async fn list_accessible_to_user(pool: SqlitePool) -> Result<()> {
        let returned_cameras = Camera::list_accessible_to_user(&pool, 3).await?;

        assert_eq!(returned_cameras.len(), 2);

        assert_eq!(returned_cameras.first().unwrap().camera_id, 1);
        assert_eq!(returned_cameras.first().unwrap().camera_name, "Front Door");
        assert!(returned_cameras.first().unwrap().can_view);
        assert!(!returned_cameras.first().unwrap().can_control);

        assert_eq!(returned_cameras.get(1).unwrap().camera_id, 2);
        assert_eq!(returned_cameras.get(1).unwrap().camera_name, "Kitchen");
        assert!(!returned_cameras.get(1).unwrap().can_view);
        assert!(!returned_cameras.get(1).unwrap().can_control);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("cameras")))]
    async fn get_using_ip(pool: SqlitePool) -> Result<()> {
        let ip_address = "127.0.0.1:40000".to_string();

        let returned_camera = Camera::get_using_ip(&pool, ip_address.clone()).await?;

        assert_eq!(returned_camera.camera_id, 1);
        assert_eq!(returned_camera.name, "Front Door");
        assert_eq!(returned_camera.ip_address, Some(ip_address));
        assert!(returned_camera.is_active);

        Ok(())
    }
}
