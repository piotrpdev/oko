use futures_util::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub use crate::web::{ApiChannelMessage, App, ImageContainer};

mod db;
mod users;
mod web;

pub use {
    db::Camera, db::CameraPermission, db::CameraPermissionUserView, db::CameraPermissionView,
    db::CameraSetting, db::CameraSettingNoMeta, db::Model, db::User, db::Video,
    db::VideoCameraView,
};

// Taken from https://github.com/hyperium/hyper/issues/2787#issuecomment-1073229886
/// Run a stream until it completes or we receive the shutdown signal.
///
/// Uses the `async-stream` to make things easier to write.
pub fn or_until_shutdown<S>(
    stream: S,
    shutdown_token: CancellationToken,
) -> impl Stream<Item = S::Item>
where
    S: Stream,
{
    async_stream::stream! {
        futures_util::pin_mut!(stream);

        loop {
            tokio::select! {
                Some(item) = stream.next() => {
                    yield item
                }
                () = shutdown_token.cancelled() => {
                    break;
                }
            }
        }
    }
}

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
