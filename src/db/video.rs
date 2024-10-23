use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::db::VideoCameraView;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Video {
    pub video_id: i64,
    pub camera_id: i64,
    pub file_path: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub file_size: Option<i64>,
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