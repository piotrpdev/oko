use serde::{Deserialize, Serialize};
use sqlx::{Result, SqlitePool};

use super::{CameraPermissionUserView, Model};

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
    pub can_control: bool,
}

impl Model for CameraPermission {
    type Default = Default;
    const DEFAULT: Default = Default {
        permission_id: -1,
        can_view: true,
        can_control: false,
    };

    async fn create_using_self(&mut self, pool: &SqlitePool) -> Result<()> {
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

        Ok(())
    }

    async fn get_using_id(pool: &SqlitePool, permission_id: i64) -> Result<Self> {
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

    async fn update_using_self(&self, pool: &SqlitePool) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE camera_permissions
            SET can_view = ?, can_control = ?
            WHERE permission_id = ?
            RETURNING permission_id
            "#,
            self.can_view,
            self.can_control,
            self.permission_id
        )
        .fetch_one(pool)
        .await?;

        Ok(())
    }

    async fn delete_using_id(pool: &SqlitePool, permission_id: i64) -> Result<()> {
        sqlx::query!(
            r#"
            DELETE
            FROM camera_permissions
            WHERE permission_id = ?
            RETURNING permission_id
            "#,
            permission_id
        )
        .fetch_one(pool)
        .await?;

        Ok(())
    }
}

impl CameraPermission {
    pub async fn list_for_camera(pool: &SqlitePool, camera_id: i64) -> Result<Vec<Self>> {
        sqlx::query_as!(
            CameraPermission,
            r#"
            SELECT *
            FROM camera_permissions
            WHERE camera_id = ?
            "#,
            camera_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn list_for_camera_with_username(
        pool: &SqlitePool,
        camera_id: i64,
    ) -> Result<Vec<CameraPermissionUserView>> {
        sqlx::query_as!(
            CameraPermissionUserView,
            r#"
            SELECT cp.permission_id, cp.camera_id, cp.user_id, u.username, cp.can_view, cp.can_control
            FROM camera_permissions cp
            JOIN users u ON cp.user_id = u.user_id
            WHERE cp.camera_id = ?
            "#,
            camera_id
        )
        .fetch_all(pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(fixtures(
        path = "../../fixtures",
        scripts("users", "cameras", "camera_permissions")
    ))]
    async fn create(pool: SqlitePool) -> Result<()> {
        let mut camera_permission = CameraPermission {
            permission_id: CameraPermission::DEFAULT.permission_id,
            camera_id: 1,
            user_id: 1,
            can_view: true,
            can_control: false,
        };

        camera_permission.create_using_self(&pool).await?;

        assert_eq!(camera_permission.permission_id, 9);

        let returned_permission =
            CameraPermission::get_using_id(&pool, camera_permission.permission_id).await?;

        assert_eq!(returned_permission.camera_id, camera_permission.camera_id);
        assert_eq!(returned_permission.user_id, camera_permission.user_id);
        assert_eq!(returned_permission.can_view, camera_permission.can_view);
        assert_eq!(
            returned_permission.can_control,
            camera_permission.can_control
        );

        Ok(())
    }

    #[sqlx::test(fixtures(
        path = "../../fixtures",
        scripts("users", "cameras", "camera_permissions")
    ))]
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

    #[sqlx::test(fixtures(
        path = "../../fixtures",
        scripts("users", "cameras", "camera_permissions")
    ))]
    async fn update(pool: SqlitePool) -> Result<()> {
        let old_camera_permission = CameraPermission::get_using_id(&pool, 1).await?;

        let new_camera_permission = CameraPermission {
            permission_id: old_camera_permission.permission_id,
            camera_id: old_camera_permission.camera_id,
            user_id: old_camera_permission.user_id,
            can_view: false,
            can_control: true,
        };

        let updated = new_camera_permission.update_using_self(&pool).await;

        assert!(updated.is_ok());

        let returned_permission =
            CameraPermission::get_using_id(&pool, old_camera_permission.permission_id).await?;

        assert_eq!(
            returned_permission.camera_id,
            new_camera_permission.camera_id
        );
        assert_eq!(returned_permission.user_id, new_camera_permission.user_id);
        assert_eq!(returned_permission.can_view, new_camera_permission.can_view);
        assert_eq!(
            returned_permission.can_control,
            new_camera_permission.can_control
        );

        Ok(())
    }

    #[sqlx::test(fixtures(
        path = "../../fixtures",
        scripts("users", "cameras", "camera_permissions")
    ))]
    async fn delete(pool: SqlitePool) -> Result<()> {
        let permission_id = 1;

        let deleted = CameraPermission::delete_using_id(&pool, permission_id).await;

        assert!(deleted.is_ok());

        let returned_permission = CameraPermission::get_using_id(&pool, permission_id).await;

        assert!(returned_permission.is_err());

        let impossible_deleted = CameraPermission::delete_using_id(&pool, permission_id).await;

        assert!(impossible_deleted.is_err());

        Ok(())
    }

    #[sqlx::test(fixtures(
        path = "../../fixtures",
        scripts("users", "cameras", "camera_permissions")
    ))]
    async fn list_for_camera(pool: SqlitePool) -> Result<()> {
        let camera_id = 1;

        let returned_permissions = CameraPermission::list_for_camera(&pool, camera_id).await?;
        assert_eq!(returned_permissions.len(), 4);

        let permission_ids: Vec<i64> = returned_permissions
            .iter()
            .map(|permission| permission.permission_id)
            .collect();

        assert!(permission_ids.contains(&1));
        assert!(permission_ids.contains(&3));
        assert!(permission_ids.contains(&5));
        assert!(permission_ids.contains(&7));

        Ok(())
    }

    #[sqlx::test(fixtures(
        path = "../../fixtures",
        scripts("users", "cameras", "camera_permissions")
    ))]
    async fn list_for_camera_with_username(pool: SqlitePool) -> Result<()> {
        let camera_id = 1;

        let returned_permissions =
            CameraPermission::list_for_camera_with_username(&pool, camera_id).await?;
        assert_eq!(returned_permissions.len(), 4);

        let permission_ids: Vec<i64> = returned_permissions
            .iter()
            .map(|permission| permission.permission_id)
            .collect();

        assert!(permission_ids.contains(&1));
        assert!(permission_ids.contains(&3));
        assert!(permission_ids.contains(&5));
        assert!(permission_ids.contains(&7));

        let usernames: Vec<&str> = returned_permissions
            .iter()
            .map(|permission| permission.username.as_str())
            .collect();

        assert!(usernames.contains(&"piotrpdev"));
        assert!(usernames.contains(&"joedaly"));
        assert!(usernames.contains(&"admin"));
        assert!(usernames.contains(&"guest"));

        Ok(())
    }
}
