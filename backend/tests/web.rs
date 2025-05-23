use std::path::PathBuf;

use futures_util::SinkExt;
use oko::{CameraPermission, CameraSetting, Model, Video};
use opencv::{
    core::{Mat, MatTraitConstManual},
    imgcodecs::{imdecode, IMREAD_COLOR},
    videoio::{
        VideoCapture, VideoCaptureTrait, VideoCaptureTraitConst, CAP_ANY, CAP_PROP_FRAME_COUNT,
    },
};
use playwright::api::{
    frame::FrameState,
    page::{self},
};
use sqlx::SqlitePool;
use time::OffsetDateTime;
use tokio::time::{sleep, Duration};
use ws_utils::Message;

#[path = "./utils.rs"]
mod utils;

// TODO: Add tests for the WebSocket routes
// ? Should these tests be run sequentially? Too many simultaneous instances of Chromium might be an issue.
// TODO: Add wait for every page navigation
// TODO: Try `waitForResponse` or `waitForLoadState` instead of `sleep` https://stackoverflow.com/questions/74586859/wait-for-network-idle-after-click

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
    scripts("users", "cameras", "camera_permissions", "camera_settings")
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

    page.click_builder("a[href=\"#/cameras\"]").click().await?;

    let s: String = page.eval("() => location.href").await?;
    assert_eq!(s, (addr_str.clone() + "#/cameras"));

    page.click_builder("button[aria-label=\"View Camera\"][data-camera-id=\"2\"]")
        .click()
        .await?;

    let None = page.get_attribute("img#live-feed", "src", None).await? else {
        return Err("src attribute found too early".into());
    };

    let mut ws_stream = utils::setup_ws(addr).await?;

    ws_stream.send(Message::Text("camera".to_string())).await?;

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

    page.click_builder("a[href=\"#/cameras\"]").click().await?;

    let s: String = page.eval("() => location.href").await?;
    assert_eq!(s, (addr_str.clone() + "#/cameras"));

    page.click_builder("button#user-menu-button")
        .click()
        .await?;

    if !page
        .is_visible(
            "button[aria-label=\"View Camera\"][data-camera-id=\"1\"]",
            None,
        )
        .await?
    {
        return Err("Front Door camera not found".into());
    }

    if page
        .is_visible(
            "button[aria-label=\"View Camera\"][data-camera-id=\"3\"]",
            None,
        )
        .await?
    {
        return Err("Backyard camera found too early".into());
    }

    page.click_builder("button#add-camera").click().await?;

    page.click_builder("button[type=\"submit\"]")
        .click()
        .await?;

    page.wait_for_selector_builder("button[aria-label=\"View Camera\"][data-camera-id=\"3\"]")
        .wait_for_selector()
        .await?;

    page.click_builder("button[aria-label=\"Remove Camera\"][data-camera-id=\"3\"]")
        .click()
        .await?;

    page.wait_for_selector_builder("button[aria-label=\"View Camera\"][data-camera-id=\"3\"]")
        .state(FrameState::Detached)
        .wait_for_selector()
        .await?;

    Ok(())
}

#[sqlx::test(fixtures(
    path = "../fixtures",
    scripts("users", "cameras", "camera_permissions", "videos", "camera_settings")
))]
async fn record(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_p, _context, _addr_str, addr, video_temp_dir) = utils::setup(&pool).await?;

    let video_list = Video::list_for_camera(&pool, 2).await?;
    assert_eq!(video_list.len(), 1);

    let video_path = video_temp_dir.path();
    let file_count = video_path.read_dir()?.count();

    let mut ws_stream = utils::setup_ws(addr).await?;

    ws_stream.send(Message::Text("camera".to_string())).await?;

    let sent_frame_count = 20;

    for _ in 0..sent_frame_count {
        ws_stream
            .send(Message::Binary(utils::REAL_TEST_IMG_1.into()))
            .await?;

        sleep(Duration::from_millis(80)).await;
    }

    ws_stream.close(None).await?;
    sleep(Duration::from_millis(80)).await;

    let new_video_list = Video::list_for_camera(&pool, 2).await?;
    assert_eq!(new_video_list.len(), 2);

    let new_file_count = video_path.read_dir()?.count();
    assert_eq!(new_file_count, file_count + 1);

    let Some(newest_video) = new_video_list.iter().max_by_key(|v| v.video_id) else {
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

// This test might be a bit flaky
#[sqlx::test(fixtures(
    path = "../fixtures",
    scripts("users", "cameras", "camera_permissions", "videos", "camera_settings")
))]
async fn download_video(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_p, context, addr_str, addr, video_temp_dir) = utils::setup(&pool).await?;

    let video_list = Video::list_for_camera(&pool, 2).await?;
    assert_eq!(video_list.len(), 1);

    let video_path = video_temp_dir.path();
    let file_count = video_path.read_dir()?.count();

    let mut ws_stream = utils::setup_ws(addr).await?;

    ws_stream.send(Message::Text("camera".to_string())).await?;

    let sent_frame_count = 20;

    for _ in 0..sent_frame_count {
        ws_stream
            .send(Message::Binary(utils::REAL_TEST_IMG_1.into()))
            .await?;

        sleep(Duration::from_millis(80)).await;
    }

    ws_stream.close(None).await?;
    sleep(Duration::from_millis(80)).await;

    let new_video_list = Video::list_for_camera(&pool, 2).await?;
    assert_eq!(new_video_list.len(), 2);

    let new_file_count = video_path.read_dir()?.count();
    assert_eq!(new_file_count, file_count + 1);

    let Some(newest_video) = new_video_list.iter().max_by_key(|v| v.video_id) else {
        return Err("No newest video found".into());
    };

    let created_video_path_str = newest_video.file_path.clone();
    let Some(created_video_filename) = created_video_path_str
        .split(std::path::MAIN_SEPARATOR)
        .next_back()
    else {
        return Err("No filename found".into());
    };

    let page = context.new_page().await?;
    page.goto_builder(&(addr_str.clone() + "#/login"))
        .goto()
        .await?;

    page.click_builder("button#login").click().await?;

    page.click_builder("a[href=\"#/cameras\"]").click().await?;

    let s: String = page.eval("() => location.href").await?;
    assert_eq!(s, (addr_str.clone() + "#/cameras"));

    page.click_builder("button[aria-label=\"View Camera\"][data-camera-id=\"2\"]")
        .click()
        .await?;

    page.wait_for_selector_builder("a[data-video-id=\"3\"]")
        .wait_for_selector()
        .await?;

    let (d, _) = tokio::join!(
        page.expect_event(page::EventType::Download),
        page.click_builder("a[data-video-id=\"3\"]").click()
    );

    let page::Event::Download(download) = d? else {
        return Err("No download event found".into());
    };

    assert_eq!(download.suggested_filename(), created_video_filename);

    let Some(downloaded_video_pathbuf) = download.path().await? else {
        return Err("No download path found".into());
    };
    let created_video_pathbuf = PathBuf::from(&newest_video.file_path);
    assert!(created_video_pathbuf.exists());

    let mut downloaded_video_cap =
        VideoCapture::from_file(downloaded_video_pathbuf.to_string_lossy().as_ref(), CAP_ANY)?;
    if !downloaded_video_cap.is_opened()? {
        return Err("Failed to open video file".into());
    }

    let downloaded_video_frame_count: f64 = downloaded_video_cap.get(CAP_PROP_FRAME_COUNT)?;
    let frame_count_diff = (downloaded_video_frame_count - 20.0).abs();
    // ! This is flaky, can fail sometimes
    assert!(frame_count_diff <= 3.0);

    let mut downloaded_video_frame = Mat::default();
    if !downloaded_video_cap.read(&mut downloaded_video_frame)? {
        return Err("Failed to read frame".into());
    }

    let downloaded_video_frame_data = downloaded_video_frame.data_bytes()?;

    let decoded_test_image = imdecode(utils::REAL_TEST_IMG_1, IMREAD_COLOR)?;
    let decoded_test_image_data = decoded_test_image.data_bytes()?;

    assert_eq!(
        downloaded_video_frame_data.len(),
        decoded_test_image_data.len()
    );

    Ok(())
}

#[sqlx::test(fixtures(
    path = "../fixtures",
    scripts("users", "cameras", "camera_permissions", "camera_settings")
))]
async fn home_feeds(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    let mut camera_1_ws_stream = utils::setup_ws_with_port(addr, 40000).await?;

    camera_1_ws_stream
        .send(Message::Text("camera".to_string()))
        .await?;

    camera_1_ws_stream
        .send(Message::Binary(utils::TEST_IMG_1.into()))
        .await?;

    sleep(Duration::from_millis(100)).await;

    let Some(src) = page
        .get_attribute(
            "img[alt=\"live camera feed\"][data-camera-id=\"1\"]",
            "src",
            None,
        )
        .await?
    else {
        return Err("No src attribute found".into());
    };

    assert!(src.contains("blob:"));

    let mut camera_2_ws_stream = utils::setup_ws_with_port(addr, 40001).await?;

    camera_2_ws_stream
        .send(Message::Text("camera".to_string()))
        .await?;

    camera_2_ws_stream
        .send(Message::Binary(utils::TEST_IMG_1.into()))
        .await?;

    sleep(Duration::from_millis(100)).await;

    let Some(new_src) = page
        .get_attribute(
            "img[alt=\"live camera feed\"][data-camera-id=\"2\"]",
            "src",
            None,
        )
        .await?
    else {
        return Err("No src attribute found".into());
    };

    assert!(new_src.contains("blob:"));
    assert_ne!(src, new_src);

    Ok(())
}

#[sqlx::test(fixtures(
    path = "../fixtures",
    scripts("users", "cameras", "camera_permissions", "videos", "camera_settings")
))]
async fn multi_camera_record(
    pool: SqlitePool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_p, _context, _addr_str, addr, video_temp_dir) = utils::setup(&pool).await?;

    let video_list = Video::list_for_camera(&pool, 2).await?;
    assert_eq!(video_list.len(), 1);

    let video_path = video_temp_dir.path();
    let file_count = video_path.read_dir()?.count();

    let mut camera_1_ws_stream = utils::setup_ws_with_port(addr, 40000).await?;
    let mut camera_2_ws_stream = utils::setup_ws_with_port(addr, 40001).await?;
    camera_1_ws_stream
        .send(Message::Text("camera".to_string()))
        .await?;
    camera_2_ws_stream
        .send(Message::Text("camera".to_string()))
        .await?;

    let sent_frame_count = 20;

    // TODO: Lower sleep time after optimizing video recording. Currently frames are being skipped too often.
    for _ in 0..sent_frame_count {
        camera_1_ws_stream
            .send(Message::Binary(utils::REAL_TEST_IMG_1.into()))
            .await?;

        sleep(Duration::from_millis(80)).await;

        camera_2_ws_stream
            .send(Message::Binary(utils::REAL_TEST_IMG_2.into()))
            .await?;

        sleep(Duration::from_millis(80)).await;
    }

    camera_1_ws_stream.close(None).await?;
    camera_2_ws_stream.close(None).await?;
    sleep(Duration::from_millis(80)).await;

    let camera_1_new_video_list = Video::list_for_camera(&pool, 1).await?;
    let camera_2_new_video_list = Video::list_for_camera(&pool, 2).await?;
    assert_eq!(camera_1_new_video_list.len(), 2);
    assert_eq!(camera_2_new_video_list.len(), 2);

    let new_file_count = video_path.read_dir()?.count();
    assert_eq!(new_file_count, file_count + 2);

    let Some(camera_1_newest_video) = camera_1_new_video_list.iter().max_by_key(|v| v.video_id)
    else {
        return Err("No newest video found".into());
    };
    let Some(camera_2_newest_video) = camera_2_new_video_list.iter().max_by_key(|v| v.video_id)
    else {
        return Err("No newest video found".into());
    };

    let camera_1_created_video_path_str = camera_1_newest_video.file_path.clone();
    let camera_1_created_video_pathbuf = PathBuf::from(&camera_1_newest_video.file_path);
    let camera_2_created_video_path_str = camera_2_newest_video.file_path.clone();
    let camera_2_created_video_pathbuf = PathBuf::from(&camera_2_newest_video.file_path);
    assert!(camera_1_created_video_pathbuf.exists());
    assert!(camera_2_created_video_pathbuf.exists());

    let mut camera_1_created_video_cap =
        VideoCapture::from_file(&camera_1_created_video_path_str, CAP_ANY)?;
    if !camera_1_created_video_cap.is_opened()? {
        return Err("Failed to open video file".into());
    }
    let mut camera_2_created_video_cap =
        VideoCapture::from_file(&camera_2_created_video_path_str, CAP_ANY)?;
    if !camera_2_created_video_cap.is_opened()? {
        return Err("Failed to open video file".into());
    }

    let camera_1_created_video_frame_count: f64 =
        camera_1_created_video_cap.get(CAP_PROP_FRAME_COUNT)?;
    let camera_1_frame_count_diff = (camera_1_created_video_frame_count - 20.0).abs();
    let camera_2_created_video_frame_count: f64 =
        camera_2_created_video_cap.get(CAP_PROP_FRAME_COUNT)?;
    let camera_2_frame_count_diff = (camera_2_created_video_frame_count - 20.0).abs();
    // ! This is flaky, can fail sometimes
    dbg!(camera_1_frame_count_diff);
    assert!(camera_1_frame_count_diff <= 3.0);
    assert!(camera_2_frame_count_diff <= 3.0);

    let mut camera_1_created_video_frame = Mat::default();
    if !camera_1_created_video_cap.read(&mut camera_1_created_video_frame)? {
        return Err("Failed to read frame".into());
    }
    let mut camera_2_created_video_frame = Mat::default();
    if !camera_2_created_video_cap.read(&mut camera_2_created_video_frame)? {
        return Err("Failed to read frame".into());
    }

    let camera_1_created_video_frame_data = camera_1_created_video_frame.data_bytes()?;
    let camera_2_created_video_frame_data = camera_2_created_video_frame.data_bytes()?;

    let camera_1_decoded_test_image = imdecode(utils::REAL_TEST_IMG_1, IMREAD_COLOR)?;
    let camera_1_decoded_test_image_data = camera_1_decoded_test_image.data_bytes()?;
    let camera_2_decoded_test_image = imdecode(utils::REAL_TEST_IMG_2, IMREAD_COLOR)?;
    let camera_2_decoded_test_image_data = camera_2_decoded_test_image.data_bytes()?;

    assert_eq!(
        camera_1_created_video_frame_data.len(),
        camera_1_decoded_test_image_data.len()
    );
    assert_eq!(
        camera_2_created_video_frame_data.len(),
        camera_2_decoded_test_image_data.len()
    );

    Ok(())
}

#[sqlx::test(fixtures(
    path = "../fixtures",
    scripts("users", "cameras", "camera_permissions")
))]
async fn camera_user_can_view(
    pool: SqlitePool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_p, context, addr_str, _addr, _video_temp_dir) = utils::setup(&pool).await?;

    let page = context.new_page().await?;
    page.goto_builder(&(addr_str.clone() + "#/login"))
        .goto()
        .await?;

    let Some(username_input) = page
        .wait_for_selector_builder("input#username")
        .wait_for_selector()
        .await?
    else {
        return Err("Username input not found".into());
    };

    username_input.fill_builder("joedaly").fill().await?;

    page.click_builder("button#login").click().await?;

    page.click_builder("button#user-menu-button")
        .click()
        .await?;

    let s: String = page.eval("() => location.href").await?;
    assert_eq!(s, (addr_str.clone() + "#/"));

    page.wait_for_selector_builder("div#logout")
        .wait_for_selector()
        .await?;

    page.click_builder("a[href=\"#/cameras\"]").click().await?;

    let s: String = page.eval("() => location.href").await?;
    assert_eq!(s, (addr_str.clone() + "#/cameras"));

    page.click_builder("button#user-menu-button")
        .click()
        .await?;

    if !page
        .is_visible(
            "button[aria-label=\"View Camera\"][data-camera-id=\"1\"]",
            None,
        )
        .await?
    {
        return Err("Front Door camera not found".into());
    }

    if page
        .is_visible(
            "button[aria-label=\"View Camera\"][data-camera-id=\"2\"]",
            None,
        )
        .await?
    {
        return Err("Kitchen camera found when it shouldn't have".into());
    }

    Ok(())
}

#[sqlx::test(fixtures(
    path = "../fixtures",
    scripts("users", "cameras", "camera_permissions")
))]
async fn camera_user_permission_updates(
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

    page.click_builder("a[href=\"#/cameras\"]").click().await?;

    let s: String = page.eval("() => location.href").await?;
    assert_eq!(s, (addr_str.clone() + "#/cameras"));

    page.click_builder("button#user-menu-button")
        .click()
        .await?;

    if !page
        .is_visible(
            "button[aria-label=\"View Camera\"][data-camera-id=\"1\"]",
            None,
        )
        .await?
    {
        return Err("Front Door camera not found".into());
    }

    page.click_builder("button[aria-label=\"Edit Camera\"][data-camera-id=\"1\"]")
        .click()
        .await?;

    let camera_permissions = CameraPermission::list_for_camera_with_username(&pool, 1).await?;
    let Some(joe_camera_permission) = camera_permissions
        .iter()
        .find(|cp| cp.username == "joedaly")
    else {
        return Err("Joe's camera permission not found".into());
    };

    assert_eq!(joe_camera_permission.permission_id, 5);
    assert!(joe_camera_permission.can_view);
    assert!(!joe_camera_permission.can_control);

    let Some(current_permission) = page
        .wait_for_selector_builder(
            "span[aria-label=\"Current User Camera Permission\"][data-permission-id=\"5\"]",
        )
        .wait_for_selector()
        .await?
    else {
        return Err("Current user camera permission not found".into());
    };

    let Some(current_permission_text) = current_permission.text_content().await? else {
        return Err("Current user camera permission text not found".into());
    };

    assert_eq!(current_permission_text, "Viewer");

    page.click_builder(
        "button[aria-label=\"Edit User Camera Permission\"][data-permission-id=\"5\"]",
    )
    .click()
    .await?;

    page.click_builder("div[data-value='\"controller\"']")
        .click()
        .await?;

    sleep(Duration::from_millis(200)).await;

    let updated_joe_camera_permission = CameraPermission::get_using_id(&pool, 5).await?;

    assert!(updated_joe_camera_permission.can_view);
    assert!(updated_joe_camera_permission.can_control);

    let Some(new_permission) = page
        .wait_for_selector_builder(
            "span[aria-label=\"Current User Camera Permission\"][data-permission-id=\"5\"]",
        )
        .wait_for_selector()
        .await?
    else {
        return Err("New user camera permission not found".into());
    };

    let Some(new_permission_text) = new_permission.text_content().await? else {
        return Err("New user camera permission text not found".into());
    };

    assert_eq!(new_permission_text, "Controller");

    Ok(())
}

// TODO: Maybe handle resolution and framerate change too
#[sqlx::test(fixtures(
    path = "../fixtures",
    scripts("users", "cameras", "camera_permissions", "camera_settings")
))]
async fn camera_setting_updates(
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

    page.click_builder("a[href=\"#/cameras\"]").click().await?;

    let s: String = page.eval("() => location.href").await?;
    assert_eq!(s, (addr_str.clone() + "#/cameras"));

    page.click_builder("button#user-menu-button")
        .click()
        .await?;

    if !page
        .is_visible(
            "button[aria-label=\"View Camera\"][data-camera-id=\"2\"]",
            None,
        )
        .await?
    {
        return Err("Kitchen camera not found".into());
    }

    page.click_builder("button[aria-label=\"Edit Camera\"][data-camera-id=\"2\"]")
        .click()
        .await?;

    let camera_setting = CameraSetting::get_for_camera(&pool, 2).await?;

    assert_eq!(camera_setting.camera_id, 2);
    assert_eq!(camera_setting.setting_id, 2);
    assert!(!camera_setting.flashlight_enabled);
    assert_eq!(camera_setting.resolution, "SVGA");
    assert_eq!(camera_setting.framerate, 5);
    assert_eq!(camera_setting.modified_by, Some(2));
    assert_eq!(
        camera_setting.last_modified,
        OffsetDateTime::from_unix_timestamp(1_729_530_145)?
    );

    let time_before_setting_update = OffsetDateTime::now_utc();

    let Some(_flashlight_setting) = page
        .wait_for_selector_builder("button[aria-label=\"Flashlight\"]")
        .wait_for_selector()
        .await?
    else {
        return Err("Flashlight setting switch not found".into());
    };

    let Some(is_flashlight_checked) = page
        .get_attribute("button[aria-label=\"Flashlight\"]", "aria-checked", None)
        .await?
    else {
        return Err("No aria-checked attribute found".into());
    };

    assert_eq!(is_flashlight_checked, "false");

    page.click_builder("button[aria-label=\"Flashlight\"]")
        .click()
        .await?;

    let Some(is_flashlight_checked_updated) = page
        .get_attribute("button[aria-label=\"Flashlight\"]", "aria-checked", None)
        .await?
    else {
        return Err("No aria-checked updated attribute found".into());
    };

    assert_eq!(is_flashlight_checked_updated, "true");

    page.click_builder("button#save-settings").click().await?;

    sleep(Duration::from_millis(200)).await;

    let updated_camera_setting = CameraSetting::get_for_camera(&pool, 2).await?;

    assert_eq!(updated_camera_setting.camera_id, 2);
    assert_eq!(updated_camera_setting.setting_id, 2);
    assert!(updated_camera_setting.flashlight_enabled);
    assert_eq!(updated_camera_setting.resolution, "SVGA");
    assert_eq!(updated_camera_setting.framerate, 5);
    assert_eq!(updated_camera_setting.modified_by, Some(1));
    assert!(updated_camera_setting.last_modified > camera_setting.last_modified);
    assert!(updated_camera_setting.last_modified >= time_before_setting_update);
    assert!(updated_camera_setting.last_modified <= OffsetDateTime::now_utc());

    page.click_builder("button[data-dialog-close]")
        .click()
        .await?;

    sleep(Duration::from_millis(100)).await;

    page.click_builder("button[aria-label=\"Edit Camera\"][data-camera-id=\"2\"]")
        .click()
        .await?;

    let Some(_flashlight_setting_again) = page
        .wait_for_selector_builder("button[aria-label=\"Flashlight\"]")
        .wait_for_selector()
        .await?
    else {
        return Err("Flashlight setting switch not found".into());
    };

    let Some(is_flashlight_checked_persistent) = page
        .get_attribute("button[aria-label=\"Flashlight\"]", "aria-checked", None)
        .await?
    else {
        return Err("No aria-checked persistent attribute found".into());
    };

    assert_eq!(is_flashlight_checked_persistent, "true");

    Ok(())
}
