use serde::{Deserialize, Serialize};
use sqlx::{Result, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraPermission {
    pub permission_id: i64,
    pub camera_id: i64,
    pub user_id: i64,
    pub can_view: bool,
    pub can_control: bool,
}

#[allow(dead_code)]
impl CameraPermission {
    pub async fn create(
        pool: &SqlitePool,
        camera_id: i64,
        user_id: i64,
        can_view: bool,
        can_control: bool,
    ) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO camera_permissions (camera_id, user_id, can_view, can_control)
            VALUES (?, ?, ?, ?)
            RETURNING permission_id
            "#,
            camera_id,
            user_id,
            can_view,
            can_control
        )
        .fetch_one(pool)
        .await?;

        Ok(result.permission_id)
    }

    pub async fn get(
        pool: &SqlitePool,
        permission_id: i64,
    ) -> Result<CameraPermission> {
        sqlx::query_as!(
            CameraPermission,
            r#"
            SELECT *
            FROM camera_permissions
            WHERE permission_id = ?
            "#,
            permission_id
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        permission_id: i64,
        can_view: bool,
        can_control: bool,
    ) -> Result<bool> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE camera_permissions
            SET can_view = ?, can_control = ?
            WHERE permission_id = ?
            "#,
            can_view,
            can_control,
            permission_id
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }

    pub async fn delete(pool: &SqlitePool, permission_id: i64) -> Result<bool> {
        let rows_affected = sqlx::query!(
            "DELETE
            FROM camera_permissions
            WHERE permission_id = ?",
            permission_id
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }
}