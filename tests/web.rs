use std::net::{Ipv4Addr, SocketAddr};

use oko::App;
use playwright::{api::BrowserContext, Playwright};
use sqlx::SqlitePool;
use tokio::net::TcpListener;

// TODO: Add tests for the WebSocket routes
// ? Should these tests be run sequentially? Too many simultaneous instances of Chromium might be an issue.

async fn setup(
    pool: SqlitePool,
) -> Result<(Playwright, BrowserContext, String), Box<dyn std::error::Error + Send + Sync>> {
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

    Ok((playwright, context, addr_str))
}

#[sqlx::test(fixtures(
    path = "../fixtures",
    scripts("users", "cameras", "camera_permissions")
))]
async fn home_redirect(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_p, context, addr_str) = setup(pool).await?;

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
    let (_p, context, addr_str) = setup(pool).await?;

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
