use std::thread::JoinHandle;

use anyhow::{bail, Context};
use embedded_svc::http::Headers;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        modem::Modem,
        prelude::Peripherals,
        task::{self, block_on},
    },
    http::{server::EspHttpServer, Method},
    io::{Read, Write},
    ipv4,
    netif::{EspNetif, NetifConfiguration, NetifStack},
    nvs::EspDefaultNvsPartition,
    timer::EspTaskTimerService,
    wifi::{AccessPointConfiguration, AsyncWifi, Configuration, EspWifi, WifiDriver},
};
use log::info;
use serde::Deserialize;

// TODO: Display possible networks to connect to
// TODO: Improve error handling
// TODO: Add more logging everywhere
// TODO: WSL / TLS / Investigate if TLS/encrypting images is too resource intensive

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

#[derive(Deserialize)]
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
    let ap = configure_ap(peripherals.modem, sys_loop.clone())?;

    block_on(async move {
        let mut wifi = start_ap(ap, &sys_loop).await?;

        let ip_info = wifi.wifi().ap_netif().get_ip_info()?;

        log::info!("Wifi AP Interface info: {:?}", ip_info);

        let _http_server = start_http_server()?;
        let _captive_portal_dns = start_dns_captive_portal()?;

        // TODO: Wait for a signal, e.g. lost connection, instead of infinitely
        wifi.wifi_wait(|_| Ok(true), None).await?;

        anyhow::Ok(())
    })?;

    Ok(())
}

fn configure_ap(modem: Modem, sys_loop: EspSystemEventLoop) -> anyhow::Result<EspWifi<'static>> {
    let nvs = EspDefaultNvsPartition::take()?;
    let wifi = WifiDriver::new(modem, sys_loop, Some(nvs))?;

    log::info!("Configuring Wifi AP..");

    let mut wifi = EspWifi::wrap_all(
        wifi,
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

    let wifi_configuration = Configuration::AccessPoint(AccessPointConfiguration {
        ssid: AP_SSID
            .try_into()
            .map_err(|()| anyhow::anyhow!("Failed to convert AP_SSID into heapless string"))?,
        channel: AP_WIFI_CHANNEL,
        max_connections: 10,
        ..Default::default()
    });
    wifi.set_configuration(&wifi_configuration)?;

    Ok(wifi)
}

async fn start_ap(
    ap: EspWifi<'static>,
    sys_loop: &EspSystemEventLoop,
) -> anyhow::Result<AsyncWifi<EspWifi<'static>>> {
    let timer_service = EspTaskTimerService::new()?;
    let mut wifi = AsyncWifi::wrap(ap, sys_loop.clone(), timer_service)?;
    wifi.start().await?;
    info!("Wifi AP started");

    wifi.wait_netif_up().await?;
    info!("Wifi AP netif up");

    let ip_info = wifi.wifi().ap_netif().get_ip_info()?;
    info!("Wi-Fi AP IP Info: {:?}", ip_info);

    info!("Created Wi-Fi");

    Ok(wifi)
}

fn start_http_server() -> anyhow::Result<EspHttpServer<'static>> {
    let mut http_server = EspHttpServer::new(&esp_idf_svc::http::server::Configuration::default())?;

    http_server
        .fn_handler("/setup.html", Method::Get, |request| {
            request
                .into_ok_response()?
                .write_all(AP_SETUP_HTML.as_bytes())
        })?
        .fn_handler("/generate_204", Method::Get, |request| {
            let location = format!("http://{AP_GATEWAY_IP}/setup.html");
            request
                .into_response(301, None, &[("Location", &location)])?
                .flush()
        })?
        .fn_handler("/gen_204", Method::Get, |request| {
            let location = format!("http://{AP_GATEWAY_IP}/setup.html");
            request
                .into_response(301, None, &[("Location", &location)])?
                .flush()
        })?
        .fn_handler::<anyhow::Error, _>("/setup.html", Method::Post, |mut req| {
            let len = req.content_len().unwrap_or(0);

            if len > AP_MAX_PAYLOAD_LEN || len == 0 {
                info!("Bad setup payload size: {}", len);
                req.into_status_response(413)?.flush()?;
                return Ok(());
            }

            let mut buf = vec![0; len.try_into()?];
            req.read_exact(&mut buf)?;

            info!(
                "Received setup form data (length: {}): {:?}",
                len,
                String::from_utf8(buf.clone())?
            );
            // TODO: Handle empty/missing values
            let form = serde_urlencoded::from_bytes::<FormData>(&buf)?;
            info!(
                "Setup form details: SSID: {}, Pass: {}, Oko: {}",
                form.ssid, form.pass, form.oko
            );
            validate_form_data(&form)?;
            info!("Form is valid");

            let location = format!("http://{AP_GATEWAY_IP}/setup.html#success");
            req.into_response(301, None, &[("Location", &location)])?
                .flush()?;

            Ok(())
        })?;

    Ok(http_server)
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
