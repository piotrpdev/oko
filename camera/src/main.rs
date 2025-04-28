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
        gpio::{self, PinDriver, Pull},
        modem::Modem,
        prelude::Peripherals,
        task::{self, block_on},
    },
    http::{server::EspHttpServer, Method},
    io::{EspIOError, Read, Write},
    ipv4,
    mdns::EspMdns,
    netif::{EspNetif, NetifConfiguration, NetifStack},
    nvs::{EspDefaultNvsPartition, EspNvs, EspNvsPartition, NvsDefault},
    sys::camera,
    timer::EspTaskTimerService,
    wifi::{AccessPointConfiguration, AsyncWifi, Configuration, EspWifi, WifiDriver},
    ws::{
        client::{
            EspWebSocketClient, EspWebSocketClientConfig, WebSocketEvent, WebSocketEventType,
        },
        FrameType,
    },
};
use log::{error, info};
use serde::Deserialize;

// TODO: Change import usage for easier reading
// TODO: Display possible networks to connect to
// TODO: Improve error handling
// TODO: Add more logging everywhere
// TODO: WSL / TLS / Investigate if TLS/encrypting images is too resource intensive
// TODO: Make messages/strings consistent
// TODO: Make pins easier to configure
// TODO: Serialize before storing to NVS instead of storing raw bytes
// TODO: Optimize CLK frequency and JPEG quality
// TODO: Provide endpoint with ESP32 real time stats
// TODO: Remove unnecessary async/block_on just use sync

const RESTART_DELAY: Duration = Duration::from_millis(500); // TODO: Find out if this is long enough

const NVS_MAX_STR_LEN: usize = 100;
const DEFAULT_RESOLUTION_STR: &str = "SVGA";
const VALID_RESOLUTIONS: [&str; 2] = [DEFAULT_RESOLUTION_STR, "VGA"];

const PREFERENCES_RESET_LIGHT_DURATION: Duration = Duration::from_millis(1000);
const PREFERENCES_NAMESPACE: &str = "preferences";
const PREFERENCES_KEY_SSID: &str = "ssid";
const PREFERENCES_KEY_PASS: &str = "pass";
const PREFERENCES_KEY_OKO: &str = "oko";

const CAMERA_SETTINGS_NAMESPACE: &str = "cam_settings";
const CAMERA_SETTINGS_KEY_FLASHLIGHT_ENABLED: &str = "flash_enabled";
const CAMERA_SETTINGS_KEY_FRAMERATE: &str = "framerate";
const CAMERA_SETTINGS_KEY_RESOLUTION: &str = "resolution";

const VFS_MAX_FDS: usize = 5;

const AP_SSID: &str = "ESP32-CAM";
const AP_GATEWAY_IP: std::net::Ipv4Addr = core::net::Ipv4Addr::new(192, 168, 1, 1);
const AP_WIFI_CHANNEL: u8 = 11;
const AP_CAPTIVE_PORTAL_DNS_IP: std::net::Ipv4Addr = core::net::Ipv4Addr::UNSPECIFIED;
const AP_CAPTIVE_PORTAL_DNS_PORT: u16 = 53;
const AP_CAPTIVE_PORTAL_BUF_SIZE: usize = 1500;
const AP_CAPTIVE_PORTAL_DNS_TTL: Duration = Duration::from_secs(300);
const AP_SETUP_HTML: &str = include_str!("setup.html");
const AP_MAX_PAYLOAD_LEN: u64 = 256; // ! This might be too low, some browsers send huge payloads

const WS_TIMEOUT: Duration = Duration::from_secs(10);

const CAMERA_ANY_PORT_INDICATOR_TEXT: &str = "camera_any_port";
const CAMERA_DEFAULT_XCLK_FREQ: i32 = 8 * 1_000_000;
const CAMERA_DEFAULT_JPG_QUALITY: i32 = 12;
const CAMERA_DEFAULT_FB_COUNT: usize = 2;
const CAMERA_DEFAULT_GRAB_MODE: camera::camera_grab_mode_t =
    camera::camera_grab_mode_t_CAMERA_GRAB_LATEST;
const CAMERA_DEFAULT_FRAME_SIZE: camera::framesize_t = camera::framesize_t_FRAMESIZE_SVGA;

// TODO: Use single shared definition for both camera and backend
#[derive(Debug, Clone, Deserialize)]
pub struct CameraSettingNoMeta {
    pub flashlight_enabled: bool,
    pub resolution: String,
    pub framerate: i64,
}

// TODO: Use single shared definition for both camera and backend
#[derive(Deserialize, Debug, Clone)]
pub enum CameraMessage {
    SettingChanged(CameraSettingNoMeta),
    Restart,
}

#[derive(Deserialize, Debug)]
struct SetupFormData {
    ssid: String,
    pass: String,
    oko: String,
}

// TODO: Require browser to send randomly generated passcode that will be used as auth for later
#[derive(Deserialize, Debug)]
struct MdnsFormData {
    oko: String,
}

#[allow(clippy::too_many_lines)] // TODO: Split into smaller functions
fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    let _mounted_eventfs = esp_idf_svc::io::vfs::MountedEventfs::mount(VFS_MAX_FDS)?;

    info!("Staring Oko camera...");

    let mut peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs_default_partition = EspDefaultNvsPartition::take()?;

    // TODO: See if not leaking memory here is possible
    let mut lamp_pin_leak = PinDriver::output(Box::leak(Box::new(peripherals.pins.gpio4)))?;
    {
        let mut factory_reset_pin = PinDriver::input(&mut peripherals.pins.gpio0)?;
        factory_reset_pin.set_pull(Pull::Down)?;

        if factory_reset_pin.is_low() {
            info!("factory_reset_pin is low, resetting preferences and camera settings...");
            lamp_pin_leak.set_high()?;

            clear_setup_details(&nvs_default_partition)?;
            clear_camera_settings(&nvs_default_partition)?;

            std::thread::sleep(PREFERENCES_RESET_LIGHT_DURATION);
            lamp_pin_leak.set_low()?;
        }
    }
    let lamp_pin = Arc::new(Mutex::new(lamp_pin_leak));

    let setup_details = get_setup_details(&nvs_default_partition)?;
    let esp_needs_setup = setup_details.ssid.is_empty() || setup_details.pass.is_empty();

    let saved_camera_settings = get_camera_settings(&nvs_default_partition)?;

    // ? Maybe move this whole thing to another thread instead of blocking the main one
    block_on(async move {
        let mut wifi;
        let _captive_portal_dns;
        let _ws_client: WebSocketClient;

        apply_camera_settings(&lamp_pin, &saved_camera_settings)?;

        let frame_size = match saved_camera_settings.resolution.as_str() {
            "VGA" => camera::framesize_t_FRAMESIZE_VGA,
            "SVGA" => camera::framesize_t_FRAMESIZE_SVGA,
            _ => CAMERA_DEFAULT_FRAME_SIZE,
        };

        info!("Initializing camera");
        let cam = Camera::new(
            peripherals.pins.gpio32,
            peripherals.pins.gpio0,
            peripherals.pins.gpio26,
            peripherals.pins.gpio27,
            peripherals.pins.gpio5,
            peripherals.pins.gpio18,
            peripherals.pins.gpio19,
            peripherals.pins.gpio21,
            peripherals.pins.gpio36,
            peripherals.pins.gpio39,
            peripherals.pins.gpio34,
            peripherals.pins.gpio35,
            peripherals.pins.gpio25,
            peripherals.pins.gpio23,
            peripherals.pins.gpio22,
            CAMERA_DEFAULT_XCLK_FREQ,
            CAMERA_DEFAULT_JPG_QUALITY,
            CAMERA_DEFAULT_FB_COUNT,
            CAMERA_DEFAULT_GRAB_MODE,
            frame_size,
        )?;
        // ? Maybe use parking_lot instead of std::sync
        let camera: Arc<Mutex<Camera<'_>>> = Arc::new(Mutex::new(cam));

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

            if setup_details.oko.is_empty() {
                info!("Oko IP is empty, not starting WebSocket client");
            } else {
                info!("Oko IP is present, starting WebSocket client");

                _ws_client = start_websocket_client(
                    camera.clone(),
                    lamp_pin,
                    saved_camera_settings.framerate,
                    nvs_default_partition.clone(),
                    setup_details,
                )?;
            }
        }

        let _http_server = start_http_server(nvs_default_partition, esp_needs_setup, camera)?;

        let mac = wifi.wifi().ap_netif().get_mac()?;
        let _mdns = start_mdns(mac)?;

        // TODO: Wait for a signal, e.g. lost connection, instead of infinitely
        wifi.wifi_wait(|_| Ok(true), None).await?;

        anyhow::Ok(())
    })?;

    Ok(())
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

fn start_mdns(mac: [u8; 6]) -> anyhow::Result<EspMdns> {
    let mut mdns = EspMdns::take()?;

    // no need for vendor prefix
    let mac_string = mac[3..]
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<String>>()
        .join("-");

    // sudo avahi-daemon --kill && avahi-browse -a -t --resolve --no-db-lookup
    mdns.set_hostname(format!("oko_camera_{mac_string}"))?;
    mdns.add_service(
        None,
        "_http",
        "_tcp",
        esp_idf_svc::http::server::Configuration::default().http_port,
        &[],
    )?;

    Ok(mdns)
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

    let nvs_default_partition_clone = nvs_default_partition.clone();

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
            // TODO: Maybe remove this handler after oko IP has been set?
            .fn_handler::<anyhow::Error, _>("/mdns_connect", Method::Post, move |mut request| {
                // Can this be exploited?
                let len = request.content_len().unwrap_or(0);

                if len > AP_MAX_PAYLOAD_LEN || len == 0 {
                    info!("Bad mdns_connect form data payload size: {}", len);
                    request.into_status_response(413)?.flush()?;
                    return Ok(());
                }

                let mut buf = vec![0; len.try_into()?];
                request.read_exact(&mut buf)?;

                info!(
                    "Received mdns_connect form data (length: {}): {:?}",
                    len,
                    String::from_utf8(buf.clone())?
                );

                let form = serde_urlencoded::from_bytes::<MdnsFormData>(&buf)?;
                info!("Mdns form details: Oko: {}", form.oko);

                validate_oko_ip(&form.oko)?;
                info!("Oko IP is valid");

                save_oko_ip(&nvs_default_partition_clone, &form.oko)?;

                let mut response = request.into_ok_response()?;
                response.write_all(b"restarting")?;
                response.flush()?;

                info!("Restarting device...");
                std::thread::sleep(RESTART_DELAY);
                esp_idf_svc::hal::reset::restart();
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

        let form = serde_urlencoded::from_bytes::<SetupFormData>(&buf)?;
        info!(
            "Setup form details: SSID: {}, Pass: {}, Oko: {}",
            form.ssid, form.pass, form.oko
        );

        validate_form_data(&form)?;
        info!("Form is valid");

        save_setup_details(&nvs_default_partition, &form)?;

        // TODO: Test this redirect more, it fails a lot of the time
        let mut response =
            request.into_response(301, None, &[("Location", &(setup_location + "#success"))])?;
        response.write_all(b"restarting")?;
        response.flush()?;

        info!("Restarting device...");
        // No sleep here for fast user feedback.
        // std::thread::sleep(RESTART_DELAY);
        esp_idf_svc::hal::reset::restart();
    })?;

    Ok(())
}

// Oko IP e.g. 192.168.0.28:8080
// TODO: Switch to Regex, assuming it can run reliably on ESP32
// X.X.X.X:X -> XXX.XXX.XXX.XXX:XXXXX
fn validate_oko_ip(oko_str: &str) -> anyhow::Result<()> {
    let oko_param = oko_str.trim().to_string();
    if !oko_param
        .chars()
        .all(|c| c.is_ascii() && !c.is_whitespace())
    {
        bail!("Oko param contains non-ascii or whitespace characters");
    }

    // Oko IP is optional, the backend can set this later after finding the camera using mDNS
    if oko_param.is_empty() {
        return Ok(());
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

fn validate_form_data(form: &SetupFormData) -> anyhow::Result<()> {
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

    validate_oko_ip(&form.oko)?;

    Ok(())
}

fn get_setup_details(
    nvs_default_partition: &EspNvsPartition<NvsDefault>,
) -> anyhow::Result<SetupFormData> {
    info!("Getting setup details");
    let nvs = EspNvs::new(nvs_default_partition.clone(), PREFERENCES_NAMESPACE, true)?;

    let mut ssid_buffer: [u8; NVS_MAX_STR_LEN] = [0; NVS_MAX_STR_LEN];
    let mut pass_buffer: [u8; NVS_MAX_STR_LEN] = [0; NVS_MAX_STR_LEN];
    let mut oko_buffer: [u8; NVS_MAX_STR_LEN] = [0; NVS_MAX_STR_LEN];

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

    Ok(SetupFormData {
        ssid: ssid.to_string(),
        pass: pass.to_string(),
        oko: oko.to_string(),
    })
}

fn save_oko_ip(
    nvs_default_partition: &EspNvsPartition<NvsDefault>,
    oko_ip: &str,
) -> anyhow::Result<()> {
    info!("Saving Oko IP details");
    let mut nvs = EspNvs::new(nvs_default_partition.clone(), PREFERENCES_NAMESPACE, true)?;

    info!("Setting raw Oko ip data");
    nvs.set_raw(PREFERENCES_KEY_OKO, oko_ip.trim().as_bytes())?;

    Ok(())
}

fn save_setup_details(
    nvs_default_partition: &EspNvsPartition<NvsDefault>,
    form: &SetupFormData,
) -> anyhow::Result<()> {
    info!("Saving setup details");
    let mut nvs = EspNvs::new(nvs_default_partition.clone(), PREFERENCES_NAMESPACE, true)?;

    info!("Setting raw setup detail data");
    nvs.set_raw(PREFERENCES_KEY_SSID, form.ssid.trim().as_bytes())?;
    nvs.set_raw(PREFERENCES_KEY_PASS, form.pass.trim().as_bytes())?;
    nvs.set_raw(PREFERENCES_KEY_OKO, form.oko.trim().as_bytes())?;

    Ok(())
}

fn clear_setup_details(nvs_default_partition: &EspNvsPartition<NvsDefault>) -> anyhow::Result<()> {
    info!("Clearing setup details");
    let mut nvs = EspNvs::new(nvs_default_partition.clone(), PREFERENCES_NAMESPACE, true)?;

    let empty: [u8; NVS_MAX_STR_LEN] = [0; NVS_MAX_STR_LEN];

    info!("Setting raw setup detail data");
    nvs.set_raw(PREFERENCES_KEY_SSID, &empty)?;
    nvs.set_raw(PREFERENCES_KEY_PASS, &empty)?;
    nvs.set_raw(PREFERENCES_KEY_OKO, &empty)?;

    Ok(())
}

fn get_camera_settings(
    nvs_default_partition: &EspNvsPartition<NvsDefault>,
) -> anyhow::Result<CameraSettingNoMeta> {
    info!("Getting camera settings");
    let nvs = EspNvs::new(
        nvs_default_partition.clone(),
        CAMERA_SETTINGS_NAMESPACE,
        true,
    )?;

    let mut flashlight_enabled_buffer: [u8; NVS_MAX_STR_LEN] = [0; NVS_MAX_STR_LEN];
    let mut framerate_buffer: [u8; NVS_MAX_STR_LEN] = [0; NVS_MAX_STR_LEN];
    let mut resolution_buffer: [u8; NVS_MAX_STR_LEN] = [0; NVS_MAX_STR_LEN];

    info!("Getting raw camera settings data");
    nvs.get_raw(
        CAMERA_SETTINGS_KEY_FLASHLIGHT_ENABLED,
        &mut flashlight_enabled_buffer,
    )?;
    nvs.get_raw(CAMERA_SETTINGS_KEY_FRAMERATE, &mut framerate_buffer)?;
    nvs.get_raw(CAMERA_SETTINGS_KEY_RESOLUTION, &mut resolution_buffer)?;

    info!("Converting raw camera settings data to strings");
    let flashlight_enabled: bool = std::str::from_utf8(&flashlight_enabled_buffer)?
        .trim()
        .trim_matches(char::from(0))
        .parse()
        .unwrap_or(false);
    let framerate: i64 = std::str::from_utf8(&framerate_buffer)?
        .trim()
        .trim_matches(char::from(0))
        .parse()
        .unwrap_or(1);
    let mut resolution: String = std::str::from_utf8(&resolution_buffer)?
        .trim()
        .trim_matches(char::from(0))
        .to_string();

    if !VALID_RESOLUTIONS.contains(&resolution.as_str()) {
        resolution = DEFAULT_RESOLUTION_STR.to_string();
    }

    Ok(CameraSettingNoMeta {
        flashlight_enabled,
        resolution,
        framerate,
    })
}

fn save_camera_settings(
    nvs_default_partition: &EspNvsPartition<NvsDefault>,
    setting: &CameraSettingNoMeta,
) -> anyhow::Result<()> {
    info!("Saving camera settings");
    let mut nvs = EspNvs::new(
        nvs_default_partition.clone(),
        CAMERA_SETTINGS_NAMESPACE,
        true,
    )?;

    info!("Setting raw camera settings data");
    nvs.set_raw(
        CAMERA_SETTINGS_KEY_FLASHLIGHT_ENABLED,
        (if setting.flashlight_enabled {
            "true"
        } else {
            "false"
        })
        .as_bytes(),
    )?;
    nvs.set_raw(
        CAMERA_SETTINGS_KEY_FRAMERATE,
        setting.framerate.to_string().as_bytes(),
    )?;
    nvs.set_raw(
        CAMERA_SETTINGS_KEY_RESOLUTION,
        setting.resolution.as_bytes(),
    )?;

    Ok(())
}

fn clear_camera_settings(
    nvs_default_partition: &EspNvsPartition<NvsDefault>,
) -> anyhow::Result<()> {
    info!("Clearing camera settings");
    let mut nvs = EspNvs::new(
        nvs_default_partition.clone(),
        CAMERA_SETTINGS_NAMESPACE,
        true,
    )?;

    let empty: [u8; NVS_MAX_STR_LEN] = [0; NVS_MAX_STR_LEN];

    info!("Setting raw camera settings data");
    nvs.set_raw(CAMERA_SETTINGS_KEY_FLASHLIGHT_ENABLED, &empty)?;
    nvs.set_raw(CAMERA_SETTINGS_KEY_FRAMERATE, &empty)?;
    nvs.set_raw(CAMERA_SETTINGS_KEY_RESOLUTION, &empty)?;

    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn apply_camera_settings(
    lamp_pin: &Arc<Mutex<PinDriver<'static, gpio::Gpio4, gpio::Output>>>, // TODO: Use a more generic type
    setting: &CameraSettingNoMeta,
) -> anyhow::Result<()> {
    info!("Applying camera settings");

    {
        let Ok(mut lamp_pin_lock) = lamp_pin.lock() else {
            anyhow::bail!("Failed to lock lamp pin in apply_camera_settings");
        };

        if setting.flashlight_enabled {
            info!("Enabling flashlight");
            if let Err(e) = lamp_pin_lock.set_high() {
                anyhow::bail!("Failed to set lamp pin high: {:#?}", e);
            }
        } else {
            info!("Disabling flashlight");
            if let Err(e) = lamp_pin_lock.set_low() {
                anyhow::bail!("Failed to set lamp pin low: {:#?}", e);
            }
        }
    }

    // ? Maybe handle resolution and framerate here

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
    lamp_pin: Arc<Mutex<PinDriver<'static, gpio::Gpio4, gpio::Output>>>, // TODO: Use a more generic type
    framerate: i64,
    nvs_default_partition: EspNvsPartition<NvsDefault>,
    form: SetupFormData,
) -> anyhow::Result<WebSocketClient> {
    // Sets stack size to CONFIG_PTHREAD_TASK_STACK_SIZE_DEFAULT, config is not inherited across threads.
    task::thread::ThreadSpawnConfiguration::default().set()?;

    let thread_handle = std::thread::Builder::new()
        .name("websocket_client".to_string())
        .spawn(move || {
            websocket_client_task(camera, lamp_pin, framerate, nvs_default_partition, form)
        })?;

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

#[allow(clippy::significant_drop_tightening)] // Makes code more readable
#[allow(clippy::needless_pass_by_value)] // Is this possible without being annoying?
fn websocket_client_task(
    camera: Arc<Mutex<Camera<'static>>>,
    lamp_pin: Arc<Mutex<PinDriver<'static, gpio::Gpio4, gpio::Output>>>, // TODO: Use a more generic type
    framerate: i64,
    nvs_default_partition: EspNvsPartition<NvsDefault>,
    form: SetupFormData,
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
            move |event| handle_event(&lamp_pin, &nvs_default_partition, event),
        )?;

        while !ws_client.is_connected() {
            std::thread::sleep(Duration::from_millis(100));
        }

        info!("Sending camera indicator WebSocket message");
        ws_client.send(
            FrameType::Text(false),
            CAMERA_ANY_PORT_INDICATOR_TEXT.as_bytes(),
        )?;

        // TODO: Lower interval based on average time taken to capture, or maybe use a more accurate timer on a separate thread and a channel?
        #[allow(clippy::cast_sign_loss)]
        let framerate_u64: u64 = if framerate < 1 { 1 } else { framerate as u64 };
        let ws_capture_interval = Duration::from_millis(1000 / (framerate_u64));

        loop {
            info!(
                "Sleeping for {}ms before next capture",
                ws_capture_interval.as_millis()
            );
            async_timer.after(ws_capture_interval).await?;

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

// TODO: Respond to Connected and Closed WebSocket messages
fn handle_event(
    lamp_pin: &Arc<Mutex<PinDriver<'static, gpio::Gpio4, gpio::Output>>>, // TODO: Use a more generic type
    nvs_default_partition: &EspNvsPartition<NvsDefault>,
    event: &Result<WebSocketEvent, EspIOError>,
) {
    let Ok(ref ev) = *event else {
        info!("Received WebSocket event error");
        return;
    };

    info!("Received WebSocket event: {:#?}", ev.event_type);

    // TODO: See if setting changes block for too long
    if let WebSocketEventType::Text(text) = ev.event_type {
        // TODO: look into bincode (fastest?) / rmp-serde (wide support) / flatbuffers (partial deserialization)
        let Ok(camera_message) = serde_json::from_str::<CameraMessage>(text) else {
            info!("Failed to parse WebSocket text event");
            return;
        };

        info!("Received WebSocket camera_message: {:#?}", camera_message);

        if let CameraMessage::SettingChanged(ref setting) = camera_message {
            info!("Received WebSocket setting change: {:#?}", setting);

            apply_camera_settings(lamp_pin, setting).unwrap_or_else(|e| {
                error!("Failed to apply camera settings: {:#?}", e);
            });

            save_camera_settings(nvs_default_partition, setting).unwrap_or_else(|e| {
                error!("Failed to save camera settings: {:#?}", e);
            });
        }

        #[allow(clippy::equatable_if_let)] // Makes code more readable
        if let CameraMessage::Restart = camera_message {
            info!("Received WebSocket restart message, restarting...");

            std::thread::sleep(RESTART_DELAY);
            esp_idf_svc::hal::reset::restart();
        }
    }
}
