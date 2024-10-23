use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

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