use std::net::{Ipv4Addr, SocketAddr};

use futures::SinkExt;
use oko::App;
use playwright::{api::BrowserContext, Playwright};
use sqlx::SqlitePool;
use tokio::{
    net::{TcpListener, TcpStream},
    time::{sleep, Duration},
};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

// TODO: Add tests for the WebSocket routes
// ? Should these tests be run sequentially? Too many simultaneous instances of Chromium might be an issue.

const TEST_IMG_1: [u8; 1] = [1];
const TEST_IMG_2: [u8; 1] = [2];

async fn setup(
    pool: SqlitePool,
) -> Result<
    (Playwright, BrowserContext, String, SocketAddr),
    Box<dyn std::error::Error + Send + Sync>,
> {
    let playwright = Playwright::initialize().await?;
    playwright.prepare()?;
    let chromium = playwright.chromium();
    let browser = chromium.launcher().headless(true).launch().await?;
    let context = browser.context_builder().build().await?;

    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await?;
    let addr = listener.local_addr()?;
    let addr_str = format!("http://{addr}/");

    let app = App { db: pool, listener };
    tokio::spawn(app.serve());

    Ok((playwright, context, addr_str, addr))
}

async fn setup_ws(
    addr: SocketAddr,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("ws://{addr}/api/ws");
    let Ok((ws_stream, _)) = connect_async(&url).await else {
        return Err("Failed to connect to WebSocket".into());
    };

    Ok(ws_stream)
}

#[sqlx::test(fixtures(
    path = "../fixtures",
    scripts("users", "cameras", "camera_permissions")
))]
async fn home_redirect(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_p, context, addr_str, _) = setup(pool).await?;

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
    let (_p, context, addr_str, _) = setup(pool).await?;

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
    let (_p, context, addr_str, addr) = setup(pool).await?;

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

    let mut ws_stream = setup_ws(addr).await?;

    ws_stream.send(Message::Binary(TEST_IMG_1.into())).await?;

    sleep(Duration::from_millis(100)).await;

    let Some(src) = page.get_attribute("img#live-feed", "src", None).await? else {
        return Err("No src attribute found".into());
    };

    assert!(src.contains("blob:"));

    ws_stream.send(Message::Binary(TEST_IMG_2.into())).await?;

    sleep(Duration::from_millis(100)).await;

    let Some(new_src) = page.get_attribute("img#live-feed", "src", None).await? else {
        return Err("No src attribute found".into());
    };

    assert!(new_src.contains("blob:"));
    assert_ne!(src, new_src);

    Ok(())
}
