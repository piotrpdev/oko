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
