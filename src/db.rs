#![allow(unused_imports)]

use sqlx::{Result, SqlitePool};

pub use camera_permission_view::CameraPermissionView;
pub use camera_permission::CameraPermission;
pub use camera_setting::CameraSetting;
pub use camera::Camera;
pub use user::User;
pub use video_camera_view::VideoCameraView;
pub use video::Video;

mod camera_permission_view;
mod camera_permission;
mod camera_setting;
mod camera;
mod user;
mod video_camera_view;
mod video;

// TODO: Make update and delete throw an error if no rows were affected
// TODO: Don't use Result<T> for create and update and delete, use Result<()>
#[allow(dead_code)]
pub trait Model {
    type Default;
    const DEFAULT: Self::Default;

    /// Add model to database, mutate self with new id, return id
    async fn create(&mut self, pool: &SqlitePool) -> Result<i64>;

    /// Get model from database using id
    async fn get_using_id(pool: &SqlitePool, id: i64) -> Result<Self> where Self: std::marker::Sized;

    /// Update model in database, return true if at least one row was updated
    async fn update(&self, pool: &SqlitePool) -> Result<bool>;

    /// Delete model from database using id, return true if at least one row was deleted
    async fn delete_using_id(pool: &SqlitePool, id: i64) -> Result<bool>;
}
