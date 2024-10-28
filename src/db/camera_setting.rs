use serde::{Deserialize, Serialize};
use sqlx::{Result, SqlitePool};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraSetting {
    pub setting_id: i64,
    pub camera_id: i64,
    pub flashlight_enabled: bool,
    pub resolution: String,
    pub framerate: i64,
    pub last_modified: OffsetDateTime,
    pub modified_by: Option<i64>,
}

impl CameraSetting {
    pub async fn create(
        pool: &SqlitePool,
        camera_id: i64,
        flashlight_enabled: bool,
        resolution: &str,
        framerate: i64,
        modified_by: Option<i64>,
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
        modified_by: Option<i64>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "cameras", "camera_settings")))]
    async fn create(pool: SqlitePool) -> Result<()> {
        let camera_id = 1;
        let flashlight_enabled = true;
        let resolution = "1920x1080";
        let framerate = 30;
        let modified_by = Some(1);

        let setting_id = CameraSetting::create(
            &pool,
            camera_id,
            flashlight_enabled,
            resolution,
            framerate,
            modified_by,
        )
        .await?;

        let setting = CameraSetting::get(&pool, setting_id).await?;
        assert_eq!(setting.camera_id, camera_id);
        assert_eq!(setting.flashlight_enabled, flashlight_enabled);
        assert_eq!(setting.resolution, resolution);
        assert_eq!(setting.framerate, framerate);
        assert_eq!(setting.modified_by, modified_by);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "cameras", "camera_settings")))]
    async fn get(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
        let setting_id = 1;

        let setting = CameraSetting::get(&pool, setting_id).await?;

        assert_eq!(setting.setting_id, setting_id);
        assert_eq!(setting.camera_id, 1);
        assert!(!setting.flashlight_enabled);
        assert_eq!(setting.resolution, "800x600");
        assert_eq!(setting.framerate, 5);
        assert_eq!(setting.last_modified, OffsetDateTime::from_unix_timestamp(1729530153)?);
        assert_eq!(setting.modified_by, Some(1));

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "cameras", "camera_settings")))]
    async fn update(pool: SqlitePool) -> Result<()> {
        let setting_id = 1;
        let flashlight_enabled = true;
        let resolution = "1920x1080";
        let framerate = 30;
        let modified_by = Some(1);

        let updated = CameraSetting::update(
            &pool,
            setting_id,
            flashlight_enabled,
            resolution,
            framerate,
            modified_by,
        )
        .await?;

        assert!(updated);

        let setting = CameraSetting::get(&pool, setting_id).await?;
        assert_eq!(setting.camera_id, 1);
        assert_eq!(setting.flashlight_enabled, flashlight_enabled);
        assert_eq!(setting.resolution, resolution);
        assert_eq!(setting.framerate, framerate);
        assert_eq!(setting.modified_by, modified_by);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "cameras", "camera_settings")))]
    async fn delete(pool: SqlitePool) -> Result<()> {
        let setting_id = 1;

        let deleted = CameraSetting::delete(&pool, setting_id).await?;
        assert!(deleted);

        let setting = CameraSetting::get(&pool, setting_id).await;
        assert!(setting.is_err());

        Ok(())
    }
}