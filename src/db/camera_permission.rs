use serde::{Deserialize, Serialize};
use sqlx::{Result, SqlitePool};

use super::Model;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraPermission {
    pub permission_id: i64,
    pub camera_id: i64,
    pub user_id: i64,
    pub can_view: bool,
    pub can_control: bool,
}

pub struct Default {
    pub permission_id: i64,
    pub can_view: bool,
    pub can_control: bool
}

impl Model for CameraPermission {
    type Default = Default;
    const DEFAULT: Default = Default {
        permission_id: -1,
        can_view: true,
        can_control: false
    };

    async fn create(
        &mut self,
        pool: &SqlitePool
    ) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO camera_permissions (camera_id, user_id, can_view, can_control)
            VALUES (?, ?, ?, ?)
            RETURNING permission_id
            "#,
            self.camera_id,
            self.user_id,
            self.can_view,
            self.can_control
        )
        .fetch_one(pool)
        .await?;

        self.permission_id = result.permission_id;

        Ok(self.permission_id)
    }

    async fn get_using_id(
        pool: &SqlitePool,
        permission_id: i64,
    ) -> Result<Self> {
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

    async fn update(
        &self,
        pool: &SqlitePool
    ) -> Result<bool> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE camera_permissions
            SET can_view = ?, can_control = ?
            WHERE permission_id = ?
            "#,
            self.can_view,
            self.can_control,
            self.permission_id
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }

    async fn delete_using_id(pool: &SqlitePool, permission_id: i64) -> Result<bool> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "cameras", "camera_permissions")))]
    async fn create(pool: SqlitePool) -> Result<()> {
        let mut camera_permission = CameraPermission {
            permission_id: CameraPermission::DEFAULT.permission_id,
            camera_id: 1,
            user_id: 1,
            can_view: true,
            can_control: false,
        };

        let permission_id = camera_permission.create(&pool).await?;

        assert_eq!(camera_permission.permission_id, 7);

        let returned_permission = CameraPermission::get_using_id(&pool, permission_id).await?;

        assert_eq!(returned_permission.camera_id, camera_permission.camera_id);
        assert_eq!(returned_permission.user_id, camera_permission.user_id);
        assert_eq!(returned_permission.can_view, camera_permission.can_view);
        assert_eq!(returned_permission.can_control, camera_permission.can_control);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "cameras", "camera_permissions")))]
    async fn get(pool: SqlitePool) -> Result<()> {
        let permission_id = 1;

        let returned_permission = CameraPermission::get_using_id(&pool, permission_id).await?;

        assert_eq!(returned_permission.permission_id, permission_id);
        assert_eq!(returned_permission.camera_id, 1);
        assert_eq!(returned_permission.user_id, 1);
        assert!(returned_permission.can_view);
        assert!(returned_permission.can_control);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "cameras", "camera_permissions")))]
    async fn update(pool: SqlitePool) -> Result<()> {
        let old_camera_permission = CameraPermission::get_using_id(&pool, 1).await?;

        let new_camera_permission = CameraPermission {
            permission_id: old_camera_permission.permission_id,
            camera_id: old_camera_permission.camera_id,
            user_id: old_camera_permission.user_id,
            can_view: false,
            can_control: true,
        };

        let updated = new_camera_permission.update(&pool).await?;

        assert!(updated);

        let returned_permission = CameraPermission::get_using_id(&pool, old_camera_permission.permission_id).await?;

        assert_eq!(returned_permission.camera_id, new_camera_permission.camera_id);
        assert_eq!(returned_permission.user_id, new_camera_permission.user_id);
        assert_eq!(returned_permission.can_view, new_camera_permission.can_view);
        assert_eq!(returned_permission.can_control, new_camera_permission.can_control);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "cameras", "camera_permissions")))]
    async fn delete(pool: SqlitePool) -> Result<()> {
        let permission_id = 1;

        let deleted = CameraPermission::delete_using_id(&pool, permission_id).await?;

        assert!(deleted);

        let returned_permission = CameraPermission::get_using_id(&pool, permission_id).await;

        assert!(returned_permission.is_err());

        Ok(())
    }
}