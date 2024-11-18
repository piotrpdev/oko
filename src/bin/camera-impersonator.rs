//! Taken from <https://github.com/twistedfall/opencv-rust/blob/master/examples/video_capture_http_stream.rs>

use std::env;
use std::process::ExitCode;

use futures::SinkExt;
use opencv::core::Mat;
use opencv::prelude::*;
use opencv::videoio::{VideoCapture, VideoCaptureTraitConst, CAP_ANY};
use tokio::time::{sleep, Duration};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

const USAGE_MESSAGE: &str = "Usage: camera-impersonator <path_to_video_file>";
const SEND_INTERVAL: Duration = Duration::from_millis(1000);

#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let Some(video_arg) = args.get(1) else {
        eprintln!("{USAGE_MESSAGE}");
        return ExitCode::FAILURE;
    };

    let mut cap = VideoCapture::from_file(video_arg, CAP_ANY).unwrap();
    if !cap.is_opened().unwrap() {
        eprintln!("Failed to open video file");
        return ExitCode::FAILURE;
    }

    let mut frame = Mat::default();

    let url = "ws://localhost:3000/api/ws".to_string();
    let (mut ws_stream, _) = connect_async(&url).await.unwrap();

    loop {
        if cap.read(&mut frame).unwrap() {
            sleep(SEND_INTERVAL).await;

            let data = frame.data_bytes().unwrap();
            println!("Sending frame of size: {}", data.len());
            ws_stream
                .send(Message::Binary(data.to_vec()))
                .await
                .unwrap();
        } else {
            break;
        }
    }

    ExitCode::SUCCESS
}
