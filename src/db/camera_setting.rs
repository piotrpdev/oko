use serde::{Deserialize, Serialize};
use sqlx::{Result, SqlitePool};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraSetting {
    pub setting_id: i64,
    pub camera_id: Option<i64>,
    pub flashlight_enabled: bool,
    pub resolution: String,
    pub framerate: i64,
    pub last_modified: OffsetDateTime,
    pub modified_by: i64,
}

#[allow(dead_code)]
impl CameraSetting {
    pub async fn create(
        pool: &SqlitePool,
        camera_id: i64,
        flashlight_enabled: bool,
        resolution: &str,
        framerate: i64,
        modified_by: i64,
    ) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO camera_settings 
            (camera_id, flashlight_enabled, resolution, framerate, modified_by)
            VALUES (?, ?, ?, ?, ?)
            RETURNING setting_id
            "#,
            camera_id,
            flashlight_enabled,
            resolution,
            framerate,
            modified_by
        )
        .fetch_one(pool)
        .await?;

        Ok(result.setting_id)
    }

    pub async fn get(
        pool: &SqlitePool,
        setting_id: i64,
    ) -> Result<CameraSetting> {
        sqlx::query_as!(
            CameraSetting,
            r#"
            SELECT setting_id, camera_id, flashlight_enabled, resolution, 
                   framerate, last_modified, modified_by
            FROM camera_settings WHERE setting_id = ?
            "#,
            setting_id
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        setting_id: i64,
        flashlight_enabled: bool,
        resolution: &str,
        framerate: i64,
        modified_by: i64,
    ) -> Result<bool> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE camera_settings
            SET flashlight_enabled = ?, resolution = ?, 
                framerate = ?, modified_by = ?,
                last_modified = CURRENT_TIMESTAMP
            WHERE setting_id = ?
            "#,
            flashlight_enabled,
            resolution,
            framerate,
            modified_by,
            setting_id
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }

    pub async fn delete(pool: &SqlitePool, setting_id: i64) -> Result<bool> {
        let rows_affected = sqlx::query!("DELETE FROM camera_settings WHERE setting_id = ?", setting_id)
            .execute(pool)
            .await?
            .rows_affected();
    
        Ok(rows_affected > 0)
    }
}