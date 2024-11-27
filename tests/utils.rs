use std::net::{Ipv4Addr, SocketAddr};

use oko::App;
use playwright::{api::BrowserContext, Playwright};
use sqlx::SqlitePool;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

pub const TEST_IMG_1: [u8; 1] = [1];
pub const TEST_IMG_2: [u8; 1] = [2];

#[allow(dead_code)]
struct TestCamera {
    camera_id: i32,
    name: &'static str,
}

#[allow(dead_code)]
const TEST_CAMERA_1: TestCamera = TestCamera {
    camera_id: 1,
    name: "Front Door",
};

#[allow(dead_code)]
const TEST_CAMERA_2: TestCamera = TestCamera {
    camera_id: 2,
    name: "Kitchen",
};

#[allow(dead_code)]
const TEST_CAMERA_3: TestCamera = TestCamera {
    camera_id: 3,
    name: "Backyard",
};

pub async fn setup(
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

pub async fn setup_ws(
    addr: SocketAddr,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("ws://{addr}/api/ws");
    let Ok((ws_stream, _)) = connect_async(&url).await else {
        return Err("Failed to connect to WebSocket".into());
    };

    Ok(ws_stream)
}
