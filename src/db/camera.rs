use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::db::CameraPermissionView;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Camera {
    pub camera_id: i64,
    pub name: String,
    pub ip_address: Option<String>,
    pub last_connected: Option<DateTime<Utc>>,
    pub is_active: bool,
}

impl Camera {
    pub async fn list_accessible_to_user(
        db: &sqlx::Pool<sqlx::Sqlite>,
        user_id: i64,
    ) -> sqlx::Result<Vec<CameraPermissionView>> {
        sqlx::query_as(
            r#"
            SELECT c.camera_id, c.name as camera_name, cp.can_view, cp.can_control
            FROM cameras c
            JOIN camera_permissions cp ON c.camera_id = cp.camera_id
            WHERE cp.user_id = ? AND c.is_active = true
            "#
        )
        .bind(user_id)
        .fetch_all(db)
        .await
    }
}
