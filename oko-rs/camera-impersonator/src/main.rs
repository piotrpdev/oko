//! Taken from <https://github.com/twistedfall/opencv-rust/blob/master/examples/video_capture_http_stream.rs>

use std::env;
use std::process::ExitCode;

use futures_util::SinkExt;
use opencv::core::{Mat, Vector};
use opencv::imgcodecs::imencode_def;
use opencv::prelude::*;
use opencv::videoio::{VideoCapture, VideoCaptureTraitConst, CAP_ANY};
use tokio::time::{sleep, Duration};
use ws_utils::{IntoClientRequest, Message};

const USAGE_MESSAGE: &str =
    "Usage: camera-impersonator <send_interval_ms> <client_port> <path_to_video_file>";

#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        eprintln!("{USAGE_MESSAGE}");
        return ExitCode::FAILURE;
    }

    // TODO: Default to 80ms
    let send_interval_u64: u64 = args
        .get(1)
        .unwrap()
        .parse()
        .expect("Failed to parse send interval");
    let send_interval = Duration::from_millis(send_interval_u64);
    // TODO: Default to 40001
    let client_port: u16 = args
        .get(2)
        .unwrap()
        .parse()
        .expect("Failed to parse client port");
    let Some(video_arg) = args.get(3) else {
        eprintln!("{USAGE_MESSAGE}");
        return ExitCode::FAILURE;
    };

    let mut cap = VideoCapture::from_file(video_arg, CAP_ANY).unwrap();
    if !cap.is_opened().unwrap() {
        eprintln!("Failed to open video file");
        return ExitCode::FAILURE;
    }

    let mut frame = Mat::default();

    let url = "ws://127.0.0.1:3000/api/ws".to_string();
    let (mut ws_stream, _) =
        ws_utils::same_port_connect(url.into_client_request().unwrap(), client_port)
            .await
            .unwrap();

    ws_stream
        .send(Message::Text("camera".to_string()))
        .await
        .unwrap();

    loop {
        if cap.read(&mut frame).unwrap() {
            sleep(send_interval).await;

            let data = frame.data_bytes().unwrap();
            println!("Sending frame of size: {}", data.len());

            // ? Maybe encode on the server instead
            let mut img_vector = Vector::default();
            let _ = imencode_def(".jpg", &frame, &mut img_vector);

            // std::fs::write("jpg_from_bytes.jpg", img).expect("Failed to write image");

            ws_stream
                .send(Message::Binary(img_vector.into()))
                .await
                .unwrap();
        } else {
            break;
        }
    }

    ExitCode::SUCCESS
}
