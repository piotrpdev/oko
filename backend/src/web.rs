pub use app::App;
pub use app::AppState;
pub use app::ImageContainer;
use serde::Deserialize;
use serde::Serialize;

use crate::CameraSettingNoMeta;

// TODO: Use single shared definition for both camera and backend
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CameraMessage {
    SettingChanged(CameraSettingNoMeta),
    Restart,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CameraListChange {
    Added { camera_id: i64 },
    Removed { camera_id: i64 },
    Updated { camera_id: i64 },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ApiChannelMessage {
    CameraAction {
        camera_id: i64,
        message: CameraMessage,
    },
    CameraListChanged(CameraListChange),
    Initial,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MdnsChannelMessage {
    ServiceDiscovered { mdns_response: mdns::Response },
    Initial,
}

mod app;
mod auth;
mod protected;
