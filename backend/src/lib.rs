use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "{}=debug,axum_login=debug,tower_sessions=debug,sqlx=warn,tower_http=debug",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().compact().without_time())
        .try_init()?;

    // TODO: Properly handle errors.
    App::new().await?.serve().await?;

    Ok(())
}
