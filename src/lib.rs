use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::web::App;

mod users;
mod web;
mod db;

pub use {
    db::Camera,
    db::CameraPermission,
    db::CameraPermissionView,
    db::CameraSetting,
    db::User,
    db::Video,
    db::VideoCameraView,
};

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::registry()
        .with(EnvFilter::new(std::env::var("RUST_LOG").unwrap_or_else(
            |_| "axum_login=debug,tower_sessions=debug,sqlx=warn,tower_http=debug".into(),
        )))
        .with(tracing_subscriber::fmt::layer())
        .try_init()?;

    App::new().await?.serve().await?;

    Ok(())
}