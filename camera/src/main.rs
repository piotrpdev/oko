use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
    time::Duration,
};

use anyhow::{bail, Context};
use embedded_svc::http::Headers;
use esp_camera_rs::Camera;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        modem::Modem,
        prelude::Peripherals,
        task::{self, block_on},
    },
    http::{server::EspHttpServer, Method},
    io::{EspIOError, Read, Write},
    ipv4,
    netif::{EspNetif, NetifConfiguration, NetifStack},
    nvs::{EspDefaultNvsPartition, EspNvs, EspNvsPartition, NvsDefault},
    sys::camera,
    timer::EspTaskTimerService,
    wifi::{AccessPointConfiguration, AsyncWifi, Configuration, EspWifi, WifiDriver},
    ws::{
        client::{EspWebSocketClient, EspWebSocketClientConfig, WebSocketEvent},
        FrameType,
    },
};
use log::info;
use serde::Deserialize;

// TODO: Change import usage for easier reading
// TODO: Display possible networks to connect to
// TODO: Improve error handling
// TODO: Add more logging everywhere
// TODO: WSL / TLS / Investigate if TLS/encrypting images is too resource intensive
// TODO: Make messages/strings consistent

const PREFERENCES_MAX_STR_LEN: usize = 100;
const PREFERENCES_NAMESPACE: &str = "preferences";
const PREFERENCES_KEY_SSID: &str = "ssid";
const PREFERENCES_KEY_PASS: &str = "pass";
const PREFERENCES_KEY_OKO: &str = "oko";

const VFS_MAX_FDS: usize = 5;

const AP_SSID: &str = "ESP32-CAM";
const AP_GATEWAY_IP: std::net::Ipv4Addr = core::net::Ipv4Addr::new(192, 168, 1, 1);
const AP_WIFI_CHANNEL: u8 = 11;
const AP_CAPTIVE_PORTAL_DNS_IP: std::net::Ipv4Addr = core::net::Ipv4Addr::UNSPECIFIED;
const AP_CAPTIVE_PORTAL_DNS_PORT: u16 = 53;
const AP_CAPTIVE_PORTAL_BUF_SIZE: usize = 1500;
const AP_CAPTIVE_PORTAL_DNS_TTL: std::time::Duration = core::time::Duration::from_secs(300);
const AP_SETUP_HTML: &str = include_str!("setup.html");
const AP_MAX_PAYLOAD_LEN: u64 = 256;

const WS_TIMEOUT: Duration = Duration::from_secs(10);
const WS_CAPTURE_INTERVAL: Duration = Duration::from_millis(5000);

const CAMERA_ANY_PORT_INDICATOR_TEXT: &str = "camera_any_port";

#[derive(Deserialize, Debug)]
struct FormData {
    ssid: String,
    pass: String,
    oko: String,
}

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    let _mounted_eventfs = esp_idf_svc::io::vfs::MountedEventfs::mount(VFS_MAX_FDS)?;

    info!("Staring Oko camera...");

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs_default_partition = EspDefaultNvsPartition::take()?;

    let setup_details = get_setup_details(&nvs_default_partition)?;
    let esp_needs_setup = setup_details.ssid.is_empty()
        || setup_details.pass.is_empty()
        || setup_details.oko.is_empty();

    // ? Maybe move this whole thing to another thread instead of blocking the main one
    block_on(async move {
        let mut wifi;
        let _captive_portal_dns;
        let _ws_client: WebSocketClient;

        info!("Initializing camera");
        // ? Maybe use parking_lot instead of std::sync
        let camera: Arc<Mutex<Camera<'_>>> = Arc::new(Mutex::new(init_camera(peripherals.pins)?));

        let mut esp_wifi = create_esp_wifi(
            peripherals.modem,
            sys_loop.clone(),
            nvs_default_partition.clone(),
        )?;

        // ! Switching from AP to STA seems to keep AP DNS details, even after restart. IP and Gateway seem to update fine.
        if esp_needs_setup {
            info!("No setup details found, starting AP");

            wifi = start_ap(esp_wifi, &sys_loop).await?;

            _captive_portal_dns = start_dns_captive_portal()?;
        } else {
            info!("Setup details found, connecting to network");

            esp_wifi.set_configuration(&Configuration::Client(
                esp_idf_svc::wifi::ClientConfiguration {
                    ssid: (&setup_details.ssid.clone()[..]).try_into().map_err(|()| {
                        anyhow::anyhow!("Could not parse the given SSID into WiFi config")
                    })?,
                    password: (&setup_details.pass.clone()[..]).try_into().map_err(|()| {
                        anyhow::anyhow!("Could not parse the given password into WiFi config")
                    })?,
                    ..Default::default()
                },
            ))?;

            wifi = start_sta(esp_wifi, &sys_loop).await?;

            _ws_client = start_websocket_client(camera.clone(), setup_details)?;
        }

        let _http_server = start_http_server(nvs_default_partition, esp_needs_setup, camera)?;

        // TODO: Wait for a signal, e.g. lost connection, instead of infinitely
        wifi.wifi_wait(|_| Ok(true), None).await?;

        anyhow::Ok(())
    })?;

    Ok(())
}

fn init_camera(
    pins: esp_idf_svc::hal::gpio::Pins,
) -> anyhow::Result<esp_camera_rs::Camera<'static>> {
    let camera = Camera::new(
        pins.gpio32,
        pins.gpio0,
        pins.gpio26,
        pins.gpio27,
        pins.gpio5,
        pins.gpio18,
        pins.gpio19,
        pins.gpio21,
        pins.gpio36,
        pins.gpio39,
        pins.gpio34,
        pins.gpio35,
        pins.gpio25,
        pins.gpio23,
        pins.gpio22,
        8 * 1_000_000,
        12,
        2,
        camera::camera_grab_mode_t_CAMERA_GRAB_LATEST,
        camera::framesize_t_FRAMESIZE_SVGA,
    )?;

    Ok(camera)
}

fn create_esp_wifi(
    modem: Modem,
    sys_loop: EspSystemEventLoop,
    nvs_default_partition: EspNvsPartition<NvsDefault>,
) -> anyhow::Result<EspWifi<'static>> {
    let mut esp_wifi = EspWifi::wrap_all(
        WifiDriver::new(modem, sys_loop, Some(nvs_default_partition))?,
        EspNetif::new(NetifStack::Sta)?,
        EspNetif::new_with_conf(&NetifConfiguration {
            ip_configuration: Some(ipv4::Configuration::Router(ipv4::RouterConfiguration {
                subnet: ipv4::Subnet {
                    gateway: AP_GATEWAY_IP,
                    ..ipv4::RouterConfiguration::default().subnet
                },
                dns: Some(AP_GATEWAY_IP),
                secondary_dns: None,
                ..Default::default()
            })),
            ..NetifConfiguration::wifi_default_router()
        })?,
    )?;

    esp_wifi.set_configuration(&Configuration::AccessPoint(AccessPointConfiguration {
        ssid: AP_SSID
            .try_into()
            .map_err(|()| anyhow::anyhow!("Failed to convert AP_SSID into heapless string"))?,
        channel: AP_WIFI_CHANNEL,
        max_connections: 10,
        ..Default::default()
    }))?;

    Ok(esp_wifi)
}

async fn start_ap(
    ap: EspWifi<'static>,
    sys_loop: &EspSystemEventLoop,
) -> anyhow::Result<AsyncWifi<EspWifi<'static>>> {
    let timer_service = EspTaskTimerService::new()?;
    let mut wifi = AsyncWifi::wrap(ap, sys_loop.clone(), timer_service)?;
    wifi.start().await?;
    info!("Wi-Fi AP started");

    wifi.wait_netif_up().await?;
    info!("Wi-Fi AP netif up");

    let ip_info = wifi.wifi().ap_netif().get_ip_info()?;
    info!("Wi-Fi AP IP Info: {:?}", ip_info);

    Ok(wifi)
}

async fn start_sta(
    sta: EspWifi<'static>,
    sys_loop: &EspSystemEventLoop,
) -> anyhow::Result<AsyncWifi<EspWifi<'static>>> {
    let timer_service = EspTaskTimerService::new()?;
    let mut wifi = AsyncWifi::wrap(sta, sys_loop.clone(), timer_service)?;
    wifi.start().await?;
    info!("Wi-Fi started");

    wifi.connect().await?;
    info!("Wi-Fi connected");

    wifi.wait_netif_up().await?;
    info!("Wi-Fi netif up");

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    info!("Wi-Fi STA IP Info: {:?}", ip_info);

    Ok(wifi)
}

// ? Maybe split into two different functions for setup/no-setup
fn start_http_server(
    nvs_default_partition: EspNvsPartition<NvsDefault>,
    esp_needs_setup: bool,
    camera: Arc<Mutex<Camera<'static>>>,
) -> anyhow::Result<EspHttpServer<'static>> {
    let mut http_server = EspHttpServer::new(&esp_idf_svc::http::server::Configuration {
        uri_match_wildcard: true,
        ..Default::default()
    })?;

    // TODO: On fail redirect to error page

    if esp_needs_setup {
        info!("Adding single wildcard HTTP captive portal handler");

        http_server.fn_handler("/*", Method::Get, |request| {
            let setup_location = format!("http://{AP_GATEWAY_IP}/setup.html");

            if !["/setup.html", "/setup.html#success", "/setup.html#error"].contains(&request.uri())
            {
                request
                    .into_response(301, None, &[("Location", &setup_location)])?
                    .flush()?;

                return Ok(());
            }

            request
                .into_ok_response()?
                .write_all(AP_SETUP_HTML.as_bytes())
        })?;
    } else {
        info!("Adding HTTP handlers");

        // TODO: Add 404 handler/redirect
        #[allow(clippy::significant_drop_tightening)] // Cannot drop earlier
        http_server
            .fn_handler("/setup.html", Method::Get, |request| {
                request
                    .into_ok_response()?
                    .write_all(AP_SETUP_HTML.as_bytes())
            })?
            .fn_handler::<anyhow::Error, _>("/image", Method::Get, move |request| {
                let camera_lock = camera
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Failed to lock camera in /image handler"))?;

                let fb = camera_lock
                    .get_framebuffer()
                    .context("Failed to get framebuffer")?;
                let data = fb.data();

                let headers = [
                    ("Content-Type", "image/jpeg"),
                    ("Content-Length", &data.len().to_string()),
                ];

                let mut response = request.into_response(200, None, &headers)?;
                response.write_all(data)?;

                Ok(())
            })?;
    }

    add_setup_form_handler(&mut http_server, nvs_default_partition, esp_needs_setup)?;

    Ok(http_server)
}

fn add_setup_form_handler(
    http_server: &mut EspHttpServer<'static>,
    nvs_default_partition: EspNvsPartition<NvsDefault>,
    use_wildcard: bool,
) -> anyhow::Result<()> {
    let uri = if use_wildcard { "/*" } else { "/setup.html" };

    http_server.fn_handler::<anyhow::Error, _>(uri, Method::Post, move |mut request| {
        let setup_location = format!("http://{AP_GATEWAY_IP}/setup.html");

        if request.uri() != "/setup.html" {
            request
                .into_response(301, None, &[("Location", &setup_location)])?
                .flush()?;

            return Ok(());
        }

        // Can this be exploited?
        let len = request.content_len().unwrap_or(0);

        if len > AP_MAX_PAYLOAD_LEN || len == 0 {
            info!("Bad setup form data payload size: {}", len);
            request.into_status_response(413)?.flush()?;
            return Ok(());
        }

        let mut buf = vec![0; len.try_into()?];
        request.read_exact(&mut buf)?;

        info!(
            "Received setup form data (length: {}): {:?}",
            len,
            String::from_utf8(buf.clone())?
        );

        let form = serde_urlencoded::from_bytes::<FormData>(&buf)?;
        info!(
            "Setup form details: SSID: {}, Pass: {}, Oko: {}",
            form.ssid, form.pass, form.oko
        );

        validate_form_data(&form)?;
        info!("Form is valid");

        save_setup_details(&nvs_default_partition, &form)?;

        request
            .into_response(301, None, &[("Location", &(setup_location + "#success"))])?
            .flush()?;

        // TODO: Restart device after a delay
        info!("Restarting device...");
        esp_idf_svc::hal::reset::restart();
    })?;

    Ok(())
}

fn validate_form_data(form: &FormData) -> anyhow::Result<()> {
    // ? Maybe use <String>.chars().count() instead of .len()
    // https://paginas.fe.up.pt/~jaime/0506/SSR/802.11i-2004.pdf

    // SSID is basically arbitrary data, spec says pretty much anything is allowed (hopefully no exploit here)
    let ssid_param = form.ssid.trim().to_string();
    if !(1..=32).contains(&ssid_param.len()) {
        bail!("SSID length is invalid");
    }

    let pass_param = form.pass.trim().to_string();
    if !(8..=63).contains(&pass_param.len()) {
        bail!("Password length is invalid");
    }

    // Oko IP e.g. 192.168.0.28:8080
    // TODO: Switch to Regex, assuming it can run reliably on ESP32
    // X.X.X.X:X -> XXX.XXX.XXX.XXX:XXXXX
    let oko_param = form.oko.trim().to_string();
    if !oko_param
        .chars()
        .all(|c| c.is_ascii() && !c.is_whitespace())
    {
        bail!("Oko param contains non-ascii or whitespace characters");
    }

    if !(9..=21).contains(&oko_param.len()) {
        bail!("Oko param length is invalid");
    }

    let oko_parts: Vec<&str> = oko_param.split(':').collect();
    if oko_parts.len() != 2 {
        bail!("Splitting by colon didn't result in two parts");
    }

    let ip_parts: Vec<&str> = oko_parts
        .first()
        .context("Failed to get characters left of colon")?
        .split('.')
        .collect();
    if ip_parts.len() != 4 {
        bail!("Splitting by dot didn't result in four parts");
    }

    for part in ip_parts {
        let part: u8 = part.parse().context("Failed to parse IP part")?;
        if !(0..=255).contains(&part) {
            bail!("IP part is out of valid range");
        }
    }

    let port: u16 = oko_parts
        .get(1)
        .context("Failed to get characters right of colon")?
        .parse()
        .context("Failed to parse port")?;
    if !(1..=65535).contains(&port) {
        bail!("Port is out of valid range");
    }

    Ok(())
}

fn get_setup_details(
    nvs_default_partition: &EspNvsPartition<NvsDefault>,
) -> anyhow::Result<FormData> {
    info!("Getting setup details");
    let nvs = EspNvs::new(nvs_default_partition.clone(), PREFERENCES_NAMESPACE, true)?;

    let mut ssid_buffer: [u8; PREFERENCES_MAX_STR_LEN] = [0; PREFERENCES_MAX_STR_LEN];
    let mut pass_buffer: [u8; PREFERENCES_MAX_STR_LEN] = [0; PREFERENCES_MAX_STR_LEN];
    let mut oko_buffer: [u8; PREFERENCES_MAX_STR_LEN] = [0; PREFERENCES_MAX_STR_LEN];

    info!("Getting raw setup detail data");
    nvs.get_raw(PREFERENCES_KEY_SSID, &mut ssid_buffer)?;
    nvs.get_raw(PREFERENCES_KEY_PASS, &mut pass_buffer)?;
    nvs.get_raw(PREFERENCES_KEY_OKO, &mut oko_buffer)?;

    info!("Converting raw setup data to strings");
    let ssid = std::str::from_utf8(&ssid_buffer)?
        .trim()
        .trim_matches(char::from(0));
    let pass = std::str::from_utf8(&pass_buffer)?
        .trim()
        .trim_matches(char::from(0));
    let oko = std::str::from_utf8(&oko_buffer)?
        .trim()
        .trim_matches(char::from(0));

    Ok(FormData {
        ssid: ssid.to_string(),
        pass: pass.to_string(),
        oko: oko.to_string(),
    })
}

fn save_setup_details(
    nvs_default_partition: &EspNvsPartition<NvsDefault>,
    form: &FormData,
) -> anyhow::Result<()> {
    info!("Saving setup details");
    let mut nvs = EspNvs::new(nvs_default_partition.clone(), PREFERENCES_NAMESPACE, true)?;

    info!("Setting raw setup detail data");
    nvs.set_raw(PREFERENCES_KEY_SSID, form.ssid.trim().as_bytes())?;
    nvs.set_raw(PREFERENCES_KEY_PASS, form.pass.trim().as_bytes())?;
    nvs.set_raw(PREFERENCES_KEY_OKO, form.oko.trim().as_bytes())?;

    Ok(())
}

fn start_dns_captive_portal() -> anyhow::Result<CaptivePortalDns> {
    // Sets stack size to CONFIG_PTHREAD_TASK_STACK_SIZE_DEFAULT, config is not inherited across threads.
    task::thread::ThreadSpawnConfiguration::default().set()?;

    let thread_handle = std::thread::Builder::new()
        .name("dns_server".to_string())
        .spawn(dns_server_task)?;

    let captive_portal_dns = CaptivePortalDns {
        thread_handle: Some(thread_handle),
    };

    Ok(captive_portal_dns)
}

pub struct CaptivePortalDns {
    thread_handle: Option<JoinHandle<anyhow::Result<()>>>,
}

impl Drop for CaptivePortalDns {
    fn drop(&mut self) {
        if let Some(handle) = self.thread_handle.take() {
            // abort the thread
            let _ = handle.join();
        }
    }
}

fn dns_server_task() -> anyhow::Result<()> {
    block_on(async {
        let stack = edge_nal_std::Stack::new();
        let mut tx_buf = [0; AP_CAPTIVE_PORTAL_BUF_SIZE];
        let mut rx_buf = [0; AP_CAPTIVE_PORTAL_BUF_SIZE];

        edge_captive::io::run(
            &stack,
            core::net::SocketAddr::new(AP_CAPTIVE_PORTAL_DNS_IP.into(), AP_CAPTIVE_PORTAL_DNS_PORT),
            &mut tx_buf,
            &mut rx_buf,
            AP_GATEWAY_IP,
            AP_CAPTIVE_PORTAL_DNS_TTL,
        )
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

        Ok(())
    })
}

fn start_websocket_client(
    camera: Arc<Mutex<Camera<'static>>>,
    form: FormData,
) -> anyhow::Result<WebSocketClient> {
    // Sets stack size to CONFIG_PTHREAD_TASK_STACK_SIZE_DEFAULT, config is not inherited across threads.
    task::thread::ThreadSpawnConfiguration::default().set()?;

    let thread_handle = std::thread::Builder::new()
        .name("websocket_client".to_string())
        .spawn(move || websocket_client_task(camera, form))?;

    let websocket_client = WebSocketClient {
        thread_handle: Some(thread_handle),
    };

    Ok(websocket_client)
}

pub struct WebSocketClient {
    thread_handle: Option<JoinHandle<anyhow::Result<()>>>,
}

impl Drop for WebSocketClient {
    fn drop(&mut self) {
        if let Some(handle) = self.thread_handle.take() {
            // abort the thread
            let _ = handle.join();
        }
    }
}

// TODO: Respond to Connected and Closed messages
#[allow(clippy::significant_drop_tightening)] // Makes code more readable
#[allow(clippy::needless_pass_by_value)] // Is this possible without being annoying?
fn websocket_client_task(
    camera: Arc<Mutex<Camera<'static>>>,
    form: FormData,
) -> anyhow::Result<()> {
    block_on(async {
        info!("Starting WebSocket client");
        let timer_service = EspTaskTimerService::new()?;
        let mut async_timer = timer_service.timer_async()?;

        let ws_url = format!("ws://{}/api/ws", form.oko);
        info!("Connecting to WebSocket server at {}", ws_url);
        let mut ws_client = EspWebSocketClient::new(
            &ws_url,
            &EspWebSocketClientConfig::default(),
            WS_TIMEOUT,
            handle_event,
        )?;

        while !ws_client.is_connected() {
            std::thread::sleep(Duration::from_millis(100));
        }

        info!("Sending camera indicator WebSocket message");
        ws_client.send(
            FrameType::Text(false),
            CAMERA_ANY_PORT_INDICATOR_TEXT.as_bytes(),
        )?;

        loop {
            info!(
                "Sleeping for {}ms before next capture",
                WS_CAPTURE_INTERVAL.as_millis()
            );
            async_timer.after(WS_CAPTURE_INTERVAL).await?;

            let camera_lock = camera
                .lock()
                .map_err(|_| anyhow::anyhow!("Failed to lock camera in /image handler"))?;

            let fb = camera_lock
                .get_framebuffer()
                .context("Failed to get framebuffer")?;
            let data = fb.data();

            info!("Sending image data over WebSocket");
            ws_client.send(FrameType::Binary(false), data)?;
        }
    })
}

fn handle_event(event: &Result<WebSocketEvent, EspIOError>) {
    let Ok(ref ev) = *event else {
        info!("Received WebSocket event error");
        return;
    };

    info!("Received WebSocket event: {:#?}", ev.event_type);
}
