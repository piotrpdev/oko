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

pub struct CameraPermissionDefaults {
    pub can_view: bool,
    pub can_control: bool
}

impl CameraPermission {
    pub const DEFAULT: CameraPermissionDefaults = CameraPermissionDefaults {
        can_view: true,
        can_control: false
    };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "cameras", "camera_permissions")))]
    async fn create(pool: SqlitePool) -> Result<()> {
        let camera_id = 1;
        let user_id = 1;
        let can_view = true;
        let can_control = false;

        let permission_id = CameraPermission::create(&pool, camera_id, user_id, can_view, can_control).await?;

        let permission = CameraPermission::get(&pool, permission_id).await?;

        assert_eq!(permission.camera_id, camera_id);
        assert_eq!(permission.user_id, user_id);
        assert_eq!(permission.can_view, can_view);
        assert_eq!(permission.can_control, can_control);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "cameras", "camera_permissions")))]
    async fn get(pool: SqlitePool) -> Result<()> {
        let permission_id = 1;

        let permission = CameraPermission::get(&pool, permission_id).await?;

        assert_eq!(permission.permission_id, permission_id);
        assert_eq!(permission.camera_id, 1);
        assert_eq!(permission.user_id, 1);
        assert!(permission.can_view);
        assert!(permission.can_control);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "cameras", "camera_permissions")))]
    async fn update(pool: SqlitePool) -> Result<()> {
        let permission_id = 1;
        let can_view = false;
        let can_control = true;

        let updated = CameraPermission::update(&pool, permission_id, can_view, can_control).await?;

        assert!(updated);

        let permission = CameraPermission::get(&pool, permission_id).await?;

        assert_eq!(permission.can_view, can_view);
        assert_eq!(permission.can_control, can_control);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "cameras", "camera_permissions")))]
    async fn delete(pool: SqlitePool) -> Result<()> {
        let permission_id = 1;

        let deleted = CameraPermission::delete(&pool, permission_id).await?;

        assert!(deleted);

        let permission = CameraPermission::get(&pool, permission_id).await;

        assert!(permission.is_err());

        Ok(())
    }
}