use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// TODO: Sync this struct with the one in users.rs
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub user_id: i64,
    pub username: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Camera {
    pub camera_id: i64,
    pub name: String,
    pub ip_address: Option<String>,
    pub resolution: Option<String>,
    pub framerate: Option<i32>,
    pub last_connected: Option<DateTime<Utc>>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CameraPermission {
    pub permission_id: i64,
    pub camera_id: i64,
    pub user_id: i64,
    pub can_view: bool,
    pub can_control: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Video {
    pub video_id: i64,
    pub camera_id: i64,
    pub file_path: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub file_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CameraSetting {
    pub setting_id: i64,
    pub camera_id: i64,
    pub flashlight_enabled: bool,
    pub resolution: Option<String>,
    pub framerate: Option<i32>,
    pub last_modified: DateTime<Utc>,
    pub modified_by: i64,
}

// TODO: Add indexes

// TODO: Create an actual SQLite View
#[derive(Debug, Serialize, FromRow)]
pub struct CameraPermissionView {
    pub camera_id: i64,
    pub camera_name: String,
    pub can_view: bool,
    pub can_control: bool,
}

// TODO: Create an actual SQLite View
#[derive(Debug, Serialize, FromRow)]
pub struct VideoCameraView {
    pub video_id: i64,
    pub camera_id: i64,
    pub camera_name: String,
    pub file_path: String,
    pub file_size: Option<i64>,
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

impl Video {
    pub async fn list_for_camera(
        db: &sqlx::Pool<sqlx::Sqlite>,
        camera_id: i64,
    ) -> sqlx::Result<Vec<VideoCameraView>> {
        sqlx::query_as(
            r#"
            SELECT v.video_id, v.camera_id, c.name as camera_name, v.file_path, v.file_size
            FROM videos v
            JOIN cameras c ON v.camera_id = c.camera_id
            WHERE c.camera_id = ?
            "#
        )
        .bind(camera_id)
        .fetch_all(db)
        .await
    }
}
