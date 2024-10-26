use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::db::VideoCameraView;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Video {
    pub video_id: i64,
    pub camera_id: Option<i64>,
    pub file_path: String,
    pub start_time: OffsetDateTime,
    pub end_time: Option<OffsetDateTime>,
    pub file_size: Option<i64>,
}

impl Video {
    pub async fn list_for_camera(
        db: &sqlx::Pool<sqlx::Sqlite>,
        camera_id: i64,
    ) -> sqlx::Result<Vec<VideoCameraView>> {
        sqlx::query_as!(
            VideoCameraView,
            r#"
            SELECT v.video_id, v.camera_id, c.name as camera_name, v.file_path, v.file_size
            FROM videos v
            JOIN cameras c ON v.camera_id = c.camera_id
            WHERE c.camera_id = ?
            "#,
            camera_id
        )
        .fetch_all(db)
        .await
    }
}