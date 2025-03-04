pub use app::App;
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
pub enum ApiChannelMessage {
    CameraRelated {
        camera_id: i64,
        message: CameraMessage,
    },
    Initial,
}

mod app;
mod auth;
mod protected;
