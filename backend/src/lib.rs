use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub use crate::web::{App, ImageContainer};

mod db;
mod users;
mod web;

pub use {
    db::Camera, db::CameraPermission, db::CameraPermissionUserView, db::CameraPermissionView,
    db::CameraSetting, db::Model, db::User, db::Video, db::VideoCameraView,
};

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Improve this, log to file, etc.
    // TODO: Use tracing wherever '?' is used.
    tracing_subscriber::registry()
        .with(EnvFilter::new(std::env::var("RUST_LOG").unwrap_or_else(
            |_| "axum_login=debug,tower_sessions=debug,sqlx=warn,tower_http=debug".into(),
        )))
        .with(tracing_subscriber::fmt::layer())
        .try_init()?;

    // TODO: Properly handle errors.
    App::new().await?.serve().await?;

    Ok(())
}
