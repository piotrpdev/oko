use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CameraPermissionUserView {
    pub permission_id: i64,
    pub camera_id: i64,
    pub user_id: i64,
    pub username: String,
    pub can_view: bool,
    pub can_control: bool,
}
