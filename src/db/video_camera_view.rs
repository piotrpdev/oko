use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// TODO: Create an actual SQLite View
#[derive(Debug, Serialize, FromRow)]
pub struct VideoCameraView {
    pub video_id: i64,
    pub camera_id: i64,
    pub camera_name: String,
    pub file_path: String,
    pub file_size: Option<i64>,
}