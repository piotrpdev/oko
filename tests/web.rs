use std::path::PathBuf;

use futures::SinkExt;
use oko::Video;
use opencv::{
    core::{Mat, MatTraitConstManual},
    imgcodecs::{imdecode, IMREAD_COLOR},
    videoio::{
        VideoCapture, VideoCaptureTrait, VideoCaptureTraitConst, CAP_ANY, CAP_PROP_FRAME_COUNT,
    },
};
use playwright::api::frame::FrameState;
use sqlx::SqlitePool;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::tungstenite::Message;

#[path = "./utils.rs"]
mod utils;

// TODO: Add tests for the WebSocket routes
// ? Should these tests be run sequentially? Too many simultaneous instances of Chromium might be an issue.

#[sqlx::test(fixtures(
    path = "../fixtures",
    scripts("users", "cameras", "camera_permissions")
))]
async fn home_redirect(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_p, context, addr_str, _, _video_temp_dir) = utils::setup(&pool).await?;

    let page = context.new_page().await?;
    page.goto_builder(&addr_str).goto().await?;

    let s: String = page.eval("() => location.href").await?;
    assert_eq!(s, addr_str.clone() + "#/login");

    Ok(())
}

#[sqlx::test(fixtures(
    path = "../fixtures",
    scripts("users", "cameras", "camera_permissions")
))]
async fn login_and_logout(
    pool: SqlitePool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_p, context, addr_str, _, _video_temp_dir) = utils::setup(&pool).await?;

    let page = context.new_page().await?;
    page.goto_builder(&(addr_str.clone() + "#/login"))
        .goto()
        .await?;

    page.click_builder("button#login").click().await?;

    page.click_builder("button#user-menu-button")
        .click()
        .await?;

    let s: String = page.eval("() => location.href").await?;
    assert_eq!(s, (addr_str.clone() + "#/"));

    page.click_builder("div#logout").click().await?;

    page.wait_for_selector_builder("button#login")
        .wait_for_selector()
        .await?;

    let s: String = page.eval("() => location.href").await?;
    assert_eq!(s, (addr_str.clone() + "#/login"));

    Ok(())
}

#[sqlx::test(fixtures(
    path = "../fixtures",
    scripts("users", "cameras", "camera_permissions")
))]
async fn live_feed(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_p, context, addr_str, addr, _video_temp_dir) = utils::setup(&pool).await?;

    let page = context.new_page().await?;
    page.goto_builder(&(addr_str.clone() + "#/login"))
        .goto()
        .await?;

    page.click_builder("button#login").click().await?;

    page.click_builder("button#user-menu-button")
        .click()
        .await?;

    let s: String = page.eval("() => location.href").await?;
    assert_eq!(s, (addr_str.clone() + "#/"));

    page.wait_for_selector_builder("div#logout")
        .wait_for_selector()
        .await?;

    let None = page.get_attribute("img#live-feed", "src", None).await? else {
        return Err("src attribute found too early".into());
    };

    let mut ws_stream = utils::setup_ws(addr).await?;

    ws_stream
        .send(Message::Binary(utils::TEST_IMG_1.into()))
        .await?;

    sleep(Duration::from_millis(100)).await;

    let Some(src) = page.get_attribute("img#live-feed", "src", None).await? else {
        return Err("No src attribute found".into());
    };

    assert!(src.contains("blob:"));

    ws_stream
        .send(Message::Binary(utils::TEST_IMG_2.into()))
        .await?;

    sleep(Duration::from_millis(100)).await;

    let Some(new_src) = page.get_attribute("img#live-feed", "src", None).await? else {
        return Err("No src attribute found".into());
    };

    assert!(new_src.contains("blob:"));
    assert_ne!(src, new_src);

    Ok(())
}

#[sqlx::test(fixtures(
    path = "../fixtures",
    scripts("users", "cameras", "camera_permissions")
))]
async fn camera_add_remove(
    pool: SqlitePool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_p, context, addr_str, _addr, _video_temp_dir) = utils::setup(&pool).await?;

    let page = context.new_page().await?;
    page.goto_builder(&(addr_str.clone() + "#/login"))
        .goto()
        .await?;

    page.click_builder("button#login").click().await?;

    page.click_builder("button#user-menu-button")
        .click()
        .await?;

    let s: String = page.eval("() => location.href").await?;
    assert_eq!(s, (addr_str.clone() + "#/"));

    page.wait_for_selector_builder("div#logout")
        .wait_for_selector()
        .await?;

    if !page.is_visible("a[data-camera-id=\"1\"]", None).await? {
        return Err("Front Door camera not found".into());
    };

    if page.is_visible("a[data-camera-id=\"3\"]", None).await? {
        return Err("Backyard camera found too early".into());
    };

    page.click_builder("button#add-camera").click().await?;

    page.click_builder("button[type=\"submit\"]")
        .click()
        .await?;

    page.wait_for_selector_builder("a[data-camera-id=\"3\"]")
        .wait_for_selector()
        .await?;

    page.click_builder("button[aria-label=\"Remove Camera\"][data-camera-id=\"3\"]")
        .click()
        .await?;

    page.wait_for_selector_builder("a[data-camera-id=\"3\"]")
        .state(FrameState::Detached)
        .wait_for_selector()
        .await?;

    Ok(())
}

#[sqlx::test(fixtures(
    path = "../fixtures",
    scripts("users", "cameras", "camera_permissions", "videos")
))]
async fn record(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_p, _context, _addr_str, addr, video_temp_dir) = utils::setup(&pool).await?;

    let camera_list = Video::list_for_camera(&pool, 2).await?;
    assert_eq!(camera_list.len(), 1);

    let video_path = video_temp_dir.path();
    let file_count = video_path.read_dir()?.count();

    let mut ws_stream = utils::setup_ws(addr).await?;

    let sent_frame_count = 20;

    for _ in 0..sent_frame_count {
        ws_stream
            .send(Message::Binary(utils::REAL_TEST_IMG_1.into()))
            .await?;

        sleep(Duration::from_millis(80)).await;
    }

    ws_stream.close(None).await?;
    sleep(Duration::from_millis(80)).await;

    let new_camera_list = Video::list_for_camera(&pool, 2).await?;
    assert_eq!(new_camera_list.len(), 2);

    let new_file_count = video_path.read_dir()?.count();
    // TODO: Update to `assert_eq!(new_file_count, file_count + 1);` after adding feat to not spawn record task on every ws connection
    assert!(new_file_count > file_count);

    let Some(newest_video) = new_camera_list.iter().max_by_key(|v| v.video_id) else {
        return Err("No newest video found".into());
    };

    let created_video_path_str = newest_video.file_path.clone();
    let created_video_pathbuf = PathBuf::from(&newest_video.file_path);
    assert!(created_video_pathbuf.exists());

    let mut created_video_cap = VideoCapture::from_file(&created_video_path_str, CAP_ANY)?;
    if !created_video_cap.is_opened()? {
        return Err("Failed to open video file".into());
    }

    let created_video_frame_count: f64 = created_video_cap.get(CAP_PROP_FRAME_COUNT)?;
    let frame_count_diff = (created_video_frame_count - 20.0).abs();
    assert!(frame_count_diff <= 3.0);

    let mut created_video_frame = Mat::default();
    if !created_video_cap.read(&mut created_video_frame)? {
        return Err("Failed to read frame".into());
    }

    let created_video_frame_data = created_video_frame.data_bytes()?;

    let decoded_test_image = imdecode(utils::REAL_TEST_IMG_1, IMREAD_COLOR)?;
    let decoded_test_image_data = decoded_test_image.data_bytes()?;

    assert_eq!(
        created_video_frame_data.len(),
        decoded_test_image_data.len()
    );

    Ok(())
}
