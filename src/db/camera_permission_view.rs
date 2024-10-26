use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, FromRow)]
pub struct CameraPermissionView {
    pub camera_id: i64,
    pub camera_name: String,
    pub can_view: bool,
    pub can_control: bool,
}