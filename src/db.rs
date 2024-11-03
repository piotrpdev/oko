use sqlx::{Result, SqlitePool};

pub use camera::Camera;
pub use camera_permission::CameraPermission;
pub use camera_permission_view::CameraPermissionView;
pub use camera_setting::CameraSetting;
pub use user::User;
pub use video::Video;
pub use video_camera_view::VideoCameraView;

mod camera;
mod camera_permission;
mod camera_permission_view;
mod camera_setting;
mod user;
mod video;
mod video_camera_view;

#[allow(dead_code)]
pub trait Model {
    type Default;
    const DEFAULT: Self::Default;

    /// Add model to database using self, mutating self with the returned id
    async fn create_using_self(&mut self, pool: &SqlitePool) -> Result<()>;

    /// Get model from database using id
    async fn get_using_id(pool: &SqlitePool, id: i64) -> Result<Self>
    where
        Self: std::marker::Sized;

    /// Update model in database using self
    async fn update_using_self(&self, pool: &SqlitePool) -> Result<()>;

    /// Delete model from database using id
    async fn delete_using_id(pool: &SqlitePool, id: i64) -> Result<()>;
}
