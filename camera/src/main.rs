use std::{thread::sleep, time::Duration};

use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        gpio::{PinDriver, Pull},
        prelude::Peripherals,
    },
    http::server::EspHttpServer,
    io::Write,
    nvs::{EspDefaultNvsPartition, EspNvs},
    wifi::{AccessPointConfiguration, BlockingWifi, Configuration, EspWifi},
};
use log::info;

// const SSID: &str = "VM9493530";
// const PASSWORD: &str = "hnxZefs2abFifxad";

const AP_SSID: &str = "ESP32-CAM";
const AP_SETUP_HTML: &str = include_str!("setup.html");

const MAX_STR_LEN: usize = 100;
const PREFERENCES_NAMESPACE: &str = "preferences";
const PREFERENCES_KEY_SSID: &str = "ssid";
const PREFERENCES_KEY_PASS: &str = "pass";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("Staring Oko camera...");

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs_default_partition = EspDefaultNvsPartition::take()?;

    let mut nvs = EspNvs::new(nvs_default_partition.clone(), PREFERENCES_NAMESPACE, true)?;

    let mut lamp_pin = PinDriver::output(peripherals.pins.gpio4)?;
    let mut factory_reset_pin = PinDriver::input(peripherals.pins.gpio0)?;
    factory_reset_pin.set_pull(Pull::Down)?;

    if factory_reset_pin.is_low() {
        info!("factory_reset_pin is low, resetting preferences...");
        lamp_pin.set_high()?;

        nvs.remove(PREFERENCES_KEY_SSID)?;
        nvs.remove(PREFERENCES_KEY_PASS)?;

        sleep(Duration::from_millis(1000));
        lamp_pin.set_low()?;
    }

    // TODO: Maybe use Rust native `get_raw`/`set_raw` and serde/nanoserde. Is it worth the increased binary size?
    let mut ssid_buffer: [u8; MAX_STR_LEN] = [0; MAX_STR_LEN];
    let mut pass_buffer: [u8; MAX_STR_LEN] = [0; MAX_STR_LEN];
    let ssid = nvs.get_str(PREFERENCES_KEY_SSID, &mut ssid_buffer)?;
    let _pass = nvs.get_str(PREFERENCES_KEY_PASS, &mut pass_buffer)?;
    info!("SSID: {:?}", ssid);
    // let ssid_not_empty = ssid.is_some_and(|s| !s.is_empty());

    let mut esp_wifi = EspWifi::new(
        peripherals.modem,
        sysloop.clone(),
        Some(nvs_default_partition),
    )?;
    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sysloop)?;

    wifi.set_configuration(&Configuration::AccessPoint(AccessPointConfiguration {
        ssid: AP_SSID
            .try_into()
            .map_err(|()| "Could not parse the AP SSID into Wi-Fi config")?,
        ..AccessPointConfiguration::default()
    }))?;

    info!("Starting Wi-Fi...");

    wifi.start()?;

    // wifi.set_configuration(&Configuration::Client(ClientConfiguration {
    //     ssid: SSID
    //         .try_into()
    //         .map_err(|()| "Could not parse the given SSID into WiFi config")?,
    //     password: PASSWORD
    //         .try_into()
    //         .map_err(|()| "Could not parse the given password into WiFi config")?,
    //     ..Default::default()
    // }))?;

    // info!("Connecting to Wi-Fi...");

    // wifi.connect()?;

    info!("Waiting for DHCP lease...");

    wifi.wait_netif_up()?;

    let ip_info = wifi.wifi().ap_netif().get_ip_info()?;

    info!("Wi-Fi AP IP Info: {:?}", ip_info);

    let mut http_server = EspHttpServer::new(&esp_idf_svc::http::server::Configuration::default())?;

    http_server.fn_handler("/", esp_idf_svc::http::Method::Get, |req| {
        req.into_ok_response()?.write_all(AP_SETUP_HTML.as_bytes())
    })?;

    loop {
        sleep(Duration::from_millis(1000));
    }

    #[allow(unreachable_code)]
    Ok(())
}
