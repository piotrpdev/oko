use serde::{Deserialize, Serialize};
use sqlx::{Result, SqlitePool};
use time::OffsetDateTime;

use super::Model;

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

pub struct Default {
    pub setting_id: i64,
    pub flashlight_enabled: bool
}

impl Default {
    #[allow(clippy::unused_self)]
    pub fn last_modified(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

impl Model for CameraSetting {
    type Default = Default;
    const DEFAULT: Default = Default {
        setting_id: -1,
        flashlight_enabled: false
    };

    async fn create(
        &mut self,
        pool: &SqlitePool
    ) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO camera_settings 
            (camera_id, flashlight_enabled, resolution, framerate, last_modified, modified_by)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING setting_id
            "#,
            self.camera_id,
            self.flashlight_enabled,
            self.resolution,
            self.framerate,
            self.last_modified,
            self.modified_by
        )
        .fetch_one(pool)
        .await?;

        self.setting_id = result.setting_id;

        Ok(self.setting_id)
    }

    async fn get_using_id(
        pool: &SqlitePool,
        id: i64,
    ) -> Result<Self> {
        sqlx::query_as!(
            CameraSetting,
            r#"
            SELECT setting_id, camera_id, flashlight_enabled, resolution, 
                   framerate, last_modified, modified_by
            FROM camera_settings WHERE setting_id = ?
            "#,
            id
        )
        .fetch_one(pool)
        .await
    }

    async fn update(
        &self,
        pool: &SqlitePool
    ) -> Result<bool> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE camera_settings
            SET flashlight_enabled = ?, resolution = ?, 
                framerate = ?, last_modified = ?,
                modified_by = ?
            WHERE setting_id = ?
            "#,
            self.flashlight_enabled,
            self.resolution,
            self.framerate,
            self.last_modified,
            self.modified_by,
            self.setting_id
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
            FROM camera_settings
            WHERE setting_id = ?
            "#,
            id
        )
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
        let mut camera_setting = CameraSetting {
            setting_id: CameraSetting::DEFAULT.setting_id,
            camera_id: 1,
            flashlight_enabled: true,
            resolution: "1920x1080".to_string(),
            framerate: 30,
            last_modified: CameraSetting::DEFAULT.last_modified(),
            modified_by: Some(1),
        };

        let setting_id = camera_setting.create(&pool).await?;

        assert_eq!(camera_setting.setting_id, 3);
        
        let returned_setting = CameraSetting::get_using_id(&pool, setting_id).await?;

        assert_eq!(returned_setting.camera_id, camera_setting.camera_id);
        assert_eq!(returned_setting.flashlight_enabled, camera_setting.flashlight_enabled);
        assert_eq!(returned_setting.resolution, camera_setting.resolution);
        assert_eq!(returned_setting.framerate, camera_setting.framerate);
        assert_eq!(returned_setting.last_modified, camera_setting.last_modified);
        assert_eq!(returned_setting.modified_by, camera_setting.modified_by);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "cameras", "camera_settings")))]
    async fn get(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
        let setting_id = 1;

        let returned_setting = CameraSetting::get_using_id(&pool, setting_id).await?;

        assert_eq!(returned_setting.setting_id, setting_id);
        assert_eq!(returned_setting.camera_id, 1);
        assert!(!returned_setting.flashlight_enabled);
        assert_eq!(returned_setting.resolution, "800x600");
        assert_eq!(returned_setting.framerate, 5);
        assert_eq!(returned_setting.last_modified, OffsetDateTime::from_unix_timestamp(1_729_530_153)?);
        assert_eq!(returned_setting.modified_by, Some(1));

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "cameras", "camera_settings")))]
    async fn update(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
        let old_camera_setting = CameraSetting::get_using_id(&pool, 1).await?;

        let new_camera_setting = CameraSetting {
            setting_id: old_camera_setting.setting_id,
            camera_id: old_camera_setting.camera_id,
            flashlight_enabled: true,
            resolution: "1920x1080".to_string(),
            framerate: old_camera_setting.framerate,
            last_modified: OffsetDateTime::from_unix_timestamp(1_729_526_553)?,
            modified_by: Some(1),
        };

        let updated = new_camera_setting.update(&pool).await?;

        assert!(updated);

        let returned_setting = CameraSetting::get_using_id(&pool, old_camera_setting.setting_id).await?;
        assert_eq!(returned_setting.camera_id, 1);
        assert_eq!(returned_setting.flashlight_enabled, new_camera_setting.flashlight_enabled);
        assert_eq!(returned_setting.resolution, new_camera_setting.resolution);
        assert_eq!(returned_setting.framerate, new_camera_setting.framerate);
        assert_eq!(returned_setting.last_modified, new_camera_setting.last_modified);
        assert_eq!(returned_setting.modified_by, new_camera_setting.modified_by);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "cameras", "camera_settings")))]
    async fn delete(pool: SqlitePool) -> Result<()> {
        let setting_id = 1;

        let deleted = CameraSetting::delete_using_id(&pool, setting_id).await?;
        assert!(deleted);

        let returned_setting_result = CameraSetting::get_using_id(&pool, setting_id).await;
        assert!(returned_setting_result.is_err());

        Ok(())
    }
}