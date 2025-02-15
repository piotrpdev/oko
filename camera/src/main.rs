use std::{net::Ipv4Addr, str::FromStr, thread::JoinHandle};

use esp_idf_svc::{
    eventloop::EspSystemEventLoop, hal::{
        modem::Modem, prelude::Peripherals, task::{self, block_on}
    }, http::{server::EspHttpServer, Method}, io::Write, ipv4, netif::{EspNetif, NetifConfiguration, NetifStack}, nvs::EspDefaultNvsPartition, timer::EspTaskTimerService, wifi::{AccessPointConfiguration, AsyncWifi, Configuration, EspWifi, WifiDriver}
};
use log::info;

const GATEWAY_IP: &str = "192.168.1.1";

const AP_SETUP_HTML: &str = include_str!("setup.html");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

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

        wifi.wifi_wait(|_| Ok(true), None)
            .await?;
        
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;

    Ok(())
}

fn configure_ap(modem: Modem, sys_loop: EspSystemEventLoop) -> Result<EspWifi<'static>, Box<dyn std::error::Error>> {
    let nvs = EspDefaultNvsPartition::take()?;
    let wifi = WifiDriver::new(modem, sys_loop, Some(nvs))?;

    log::info!("Configuring Wifi AP..");
    let netmask = 24;
    let gateway_addr = Ipv4Addr::from_str(GATEWAY_IP)?;
    let wifi_channel = 11;

    let mut wifi = EspWifi::wrap_all(
        wifi,
        EspNetif::new(NetifStack::Sta)?,
        EspNetif::new_with_conf(&NetifConfiguration {
            ip_configuration: Some(ipv4::Configuration::Router(ipv4::RouterConfiguration {
                subnet: ipv4::Subnet {
                    gateway: gateway_addr,
                    mask: ipv4::Mask(netmask),
                },
                dhcp_enabled: true,
                dns: Some(gateway_addr),
                secondary_dns: None,
            })),
            ..NetifConfiguration::wifi_default_router()
        })?,
    )?;

    let wifi_configuration = Configuration::AccessPoint(AccessPointConfiguration {
        channel: wifi_channel,
        max_connections: 10,
        ..Default::default()
    });
    wifi.set_configuration(&wifi_configuration)?;
    Ok(wifi)
}

async fn start_ap(
    ap: EspWifi<'static>,
    sys_loop: &EspSystemEventLoop,
) -> Result<AsyncWifi<EspWifi<'static>>, Box<dyn std::error::Error>> {
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

fn start_http_server() -> Result<EspHttpServer<'static>, Box<dyn std::error::Error>> {
    let mut http_server = EspHttpServer::new(&esp_idf_svc::http::server::Configuration::default())?;

    http_server
        .fn_handler("/", Method::Get, |request| {
            request.into_ok_response()?.write_all(AP_SETUP_HTML.as_bytes())
        })?
        .fn_handler("/generate_204", Method::Get, |request| {
            request.into_ok_response()?.write_all(AP_SETUP_HTML.as_bytes())
        })?
        .fn_handler("/gen_204", Method::Get, |request| {
            request.into_ok_response()?.write_all(AP_SETUP_HTML.as_bytes())
        })?;

    Ok(http_server)
}

fn start_dns_captive_portal() -> Result<CaptivePortalDns, Box<dyn std::error::Error>> {
    const STACK_SIZE: usize = 16 * 1024;
    task::thread::ThreadSpawnConfiguration {
        stack_size: 16 * 1024,
        priority: 5,
        ..Default::default()
    }
    .set()?;
    let thread_handle = std::thread::Builder::new()
        .name("dns_server".to_string())
        .stack_size(STACK_SIZE)
        .spawn(dns_server_task)?;
    let captive_portal_dns = CaptivePortalDns {
        thread_handle: Some(thread_handle),
    };
    Ok(captive_portal_dns)
}

pub struct CaptivePortalDns {
    thread_handle: Option<JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>>,
}

impl Drop for CaptivePortalDns {
    fn drop(&mut self) {
        if let Some(handle) = self.thread_handle.take() {
            // abort the thread
            let _ = handle.join();
        }
    }
}

fn dns_server_task() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let gateway_addr = Ipv4Addr::from_str(GATEWAY_IP)?;
    let ttl = core::time::Duration::from_secs(300);
    let addr = core::net::SocketAddr::new(core::net::Ipv4Addr::UNSPECIFIED.into(), 53);
    let udp_socket = std::net::UdpSocket::bind(addr)?;
    log::info!("DNS server listening on {addr}...");
    let mut tx_buf = [0u8; 512];
    let mut rx_buf = [0u8; 512];
    loop {
        let (len, src) = udp_socket.recv_from(&mut rx_buf)?;
        let request = &mut rx_buf.get_mut(..len).ok_or("Invalid DNS request")?;
        log::debug!("Received DNS request from {src}...");
        let len = match edge_captive::reply(request, &gateway_addr.octets(), ttl, &mut tx_buf) {
            Ok(len) => len,
            Err(e) => match e {
                edge_captive::DnsError::InvalidMessage => {
                    log::warn!("Got invalid DNS message from {src}, skipping...");
                    continue;
                }
                other @ edge_captive::DnsError::ShortBuf => Err(format!("DNSError: {other}")),
            }?,
        };

        udp_socket.send_to(tx_buf.get_mut(..len).ok_or("Invalid DNS response")?, src)?;
        log::debug!("Sent DNS response to {src}");
    }
}
