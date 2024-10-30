use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CameraPermissionView {
    pub camera_id: i64,
    pub camera_name: String,
    pub can_view: bool,
    pub can_control: bool,
}