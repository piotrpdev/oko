use std::thread::JoinHandle;

use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        modem::Modem,
        prelude::Peripherals,
        task::{self, block_on},
    },
    http::{server::EspHttpServer, Method},
    io::Write,
    ipv4,
    netif::{EspNetif, NetifConfiguration, NetifStack},
    nvs::EspDefaultNvsPartition,
    timer::EspTaskTimerService,
    wifi::{AccessPointConfiguration, AsyncWifi, Configuration, EspWifi, WifiDriver},
};
use log::info;

const VFS_MAX_FDS: usize = 5;

const AP_SSID: &str = "ESP32-CAM";
const AP_GATEWAY_IP: std::net::Ipv4Addr = core::net::Ipv4Addr::new(192, 168, 1, 1);
const AP_WIFI_CHANNEL: u8 = 11;
const AP_CAPTIVE_PORTAL_DNS_IP: std::net::Ipv4Addr = core::net::Ipv4Addr::UNSPECIFIED;
const AP_CAPTIVE_PORTAL_DNS_PORT: u16 = 53;
const AP_CAPTIVE_PORTAL_BUF_SIZE: usize = 1500;
const AP_CAPTIVE_PORTAL_DNS_TTL: std::time::Duration = core::time::Duration::from_secs(300);
const AP_SETUP_HTML: &str = include_str!("setup.html");

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
        .fn_handler("/", Method::Get, |request| {
            request
                .into_ok_response()?
                .write_all(AP_SETUP_HTML.as_bytes())
        })?
        .fn_handler("/generate_204", Method::Get, |request| {
            request
                .into_ok_response()?
                .write_all(AP_SETUP_HTML.as_bytes())
        })?
        .fn_handler("/gen_204", Method::Get, |request| {
            request
                .into_ok_response()?
                .write_all(AP_SETUP_HTML.as_bytes())
        })?;

    Ok(http_server)
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
