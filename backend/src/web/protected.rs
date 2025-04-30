use std::sync::Arc;

use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Router,
};

use crate::users::AuthSession;
use crate::web::AppState;

pub fn router(app_state: Arc<AppState>) -> Router<()> {
    Router::new()
        .route("/api/", get(self::get::protected))
        .route("/api/cameras", get(self::get::cameras))
        .route("/api/cameras", post(self::post::cameras))
        .route("/api/cameras/:camera_id", delete(self::delete::cameras))
        .route(
            "/api/cameras/:camera_id/videos",
            get(self::get::videos_for_camera),
        )
        .route(
            "/api/cameras/:camera_id/permissions",
            get(self::get::camera_permissions),
        )
        .route("/api/videos/:video_id", get(self::get::video))
        .route(
            "/api/permissions/:permission_id",
            patch(self::patch::permissions),
        )
        .route(
            "/api/cameras/:camera_id/settings",
            get(self::get::camera_settings),
        )
        .route(
            "/api/settings/:setting_id",
            patch(self::patch::camera_settings),
        )
        .route(
            "/api/cameras/:camera_id/restart",
            post(self::post::camera_restart),
        )
        .route("/api/mdns_cameras_sse", get(self::get::mdns_cameras_sse))
        .route("/api/users", get(self::get::users))
        .route("/api/users", post(self::post::users))
        .route("/api/users/:user_id", patch(self::patch::users))
        .route("/api/users/:user_id", delete(self::delete::users))
        .with_state(app_state)
}

mod get {
    use std::{net::SocketAddr, sync::Arc, time::Duration};

    use axum::{
        body::Body,
        extract::{Path, State},
        response::{sse, Sse},
        Json,
    };
    use http::header;
    use serde::Serialize;
    use tokio_stream::{wrappers::WatchStream, StreamExt};
    use tokio_util::io::ReaderStream;
    use tracing::error;

    use crate::{
        db::Camera,
        web::{AppState, MdnsChannelMessage},
        CameraPermission, CameraPermissionView, CameraSetting, Model, User, Video,
    };

    use super::{AuthSession, IntoResponse, StatusCode};

    #[derive(Serialize)]
    struct ProtectedJson {
        user: User,
        cameras: Vec<CameraPermissionView>,
    }

    pub async fn protected(
        auth_session: AuthSession,
        state: State<Arc<AppState>>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                // TODO: Handle different error types
                let Ok(cameras) =
                    Camera::list_accessible_to_user(&state.db_pool, user.user_id).await
                else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                let safe_user = user.to_redacted_clone();

                let protected_json = ProtectedJson {
                    user: safe_user,
                    cameras,
                };

                Json(protected_json).into_response()
            }

            None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }

    pub async fn cameras(
        auth_session: AuthSession,
        state: State<Arc<AppState>>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                let Ok(cameras) =
                    Camera::list_accessible_to_user(&state.db_pool, user.user_id).await
                else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                Json(cameras).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }

    pub async fn videos_for_camera(
        auth_session: AuthSession,
        state: State<Arc<AppState>>,
        Path(camera_id): Path<i64>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                let Ok(cameras) =
                    Camera::list_accessible_to_user(&state.db_pool, user.user_id).await
                else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                if !cameras.iter().any(|c| c.camera_id == camera_id) {
                    return StatusCode::FORBIDDEN.into_response();
                }

                let Ok(videos) = Video::list_for_camera(&state.db_pool, camera_id).await else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                Json(videos).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }

    // Code copied from: https://github.com/tokio-rs/axum/discussions/608
    pub async fn video(
        auth_session: AuthSession,
        state: State<Arc<AppState>>,
        Path(video_id): Path<i64>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                let Ok(video) = Video::get_using_id(&state.db_pool, video_id).await else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                let Some(video_camera_id) = video.camera_id else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                let Ok(cameras) =
                    Camera::list_accessible_to_user(&state.db_pool, user.user_id).await
                else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                if !cameras.iter().any(|c| c.camera_id == video_camera_id) {
                    return StatusCode::FORBIDDEN.into_response();
                }

                let Ok(file) = tokio::fs::File::open(video.file_path.clone()).await else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                let Some(filename) = video.file_path.split(std::path::MAIN_SEPARATOR).next_back()
                else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };
                let content_type = "video/mp4";

                let stream = ReaderStream::new(file);
                let body = Body::from_stream(stream);

                let headers = [
                    (header::CONTENT_TYPE, content_type),
                    (
                        header::CONTENT_DISPOSITION,
                        &format!("attachment; filename={filename:?}"),
                    ),
                ];

                (headers, body).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }

    pub async fn camera_permissions(
        auth_session: AuthSession,
        state: State<Arc<AppState>>,
        Path(camera_id): Path<i64>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                if user.username != "admin" {
                    return StatusCode::FORBIDDEN.into_response();
                }

                let Ok(permissions) =
                    CameraPermission::list_for_camera_with_username(&state.db_pool, camera_id)
                        .await
                else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                Json(permissions).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }

    pub async fn camera_settings(
        auth_session: AuthSession,
        state: State<Arc<AppState>>,
        Path(camera_id): Path<i64>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(_) => {
                let Ok(settings) = CameraSetting::get_for_camera(&state.db_pool, camera_id).await
                else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                Json(settings).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }

    #[derive(Serialize)]
    struct MdnsService {
        hostname: String,
        socket_address: SocketAddr,
    }

    pub async fn mdns_cameras_sse(
        auth_session: AuthSession,
        state: State<Arc<AppState>>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                if user.username != "admin" {
                    return StatusCode::FORBIDDEN.into_response();
                }

                let mdns_channel_rx = state.mdns_channel.subscribe();
                let mdns_stream = WatchStream::from_changes(mdns_channel_rx);

                let mdns_sse_stream =
                    mdns_stream.map(|mdns_channel_message| -> Result<sse::Event, &str> {
                        match mdns_channel_message {
                            MdnsChannelMessage::ServiceDiscovered { mdns_response } => {
                                let (Some(hostname_str), Some(socket_address)) =
                                    (mdns_response.hostname(), mdns_response.socket_address())
                                else {
                                    return Err("");
                                };

                                sse::Event::default()
                                    .json_data(MdnsService {
                                        hostname: hostname_str.to_owned(),
                                        socket_address,
                                    })
                                    .map_err(|_| {
                                        error!("Failed to serialize mDNS response JSON data");
                                        ""
                                    })
                            }
                            MdnsChannelMessage::Initial => Err(""),
                        }
                    });

                let valid_mdns_sse_stream = mdns_sse_stream
                    .skip_while(|event_result: &Result<sse::Event, &str>| event_result.is_err());

                let valid_mdns_sse_stream_until_shutdown =
                    crate::or_until_shutdown(valid_mdns_sse_stream, state.shutdown_token.clone());

                Sse::new(valid_mdns_sse_stream_until_shutdown)
                    .keep_alive(
                        sse::KeepAlive::new()
                            .interval(Duration::from_secs(1))
                            .text("keep-alive-text"),
                    )
                    .into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }

    pub async fn users(
        auth_session: AuthSession,
        state: State<Arc<AppState>>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                if user.username != "admin" {
                    return StatusCode::FORBIDDEN.into_response();
                }

                let Ok(users) = User::get_all(&state.db_pool).await else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                Json(users).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }
}

// TODO: Don't always return the same error

mod post {
    use std::net::{IpAddr, SocketAddr};
    use std::sync::Arc;

    use super::{AuthSession, IntoResponse, StatusCode};
    use crate::web::{AppState, CameraListChange};
    use crate::{ApiChannelMessage, User};
    use crate::{Camera, CameraPermission, CameraSetting, Model};
    use axum::extract::{Path, State};
    use axum::Form;
    use axum::Json;
    use password_auth::generate_hash;
    use serde::Deserialize;
    use tokio::task;
    use tracing::debug;

    #[derive(Debug, Clone, Deserialize)]
    pub struct AddCameraForm {
        pub name: String,
        pub address: String,
        #[serde(default)]
        pub skip_mdns_connect: bool,
    }

    pub enum MdnsConnectAddress {
        IpAddr(IpAddr),
        SocketAddr(SocketAddr),
    }

    impl TryFrom<String> for MdnsConnectAddress {
        type Error = ();

        fn try_from(address: String) -> Result<Self, Self::Error> {
            #[allow(clippy::option_if_let_else)] // This is more readable
            if let Ok(ip_addr) = address.parse::<std::net::IpAddr>() {
                Ok(Self::IpAddr(ip_addr))
            } else if let Ok(socket_addr) = address.parse::<std::net::SocketAddr>() {
                Ok(Self::SocketAddr(socket_addr))
            } else {
                Err(())
            }
        }
    }

    #[allow(clippy::too_many_lines)] // TODO: Refactor
    pub async fn cameras(
        auth_session: AuthSession,
        state: State<Arc<AppState>>,
        Form(camera_form): Form<AddCameraForm>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                if user.username != "admin" {
                    return StatusCode::FORBIDDEN.into_response();
                }

                let Ok(mdns_connect_address) = camera_form.address.try_into() else {
                    return StatusCode::BAD_REQUEST.into_response();
                };

                let mdns_connect_url = match mdns_connect_address {
                    MdnsConnectAddress::IpAddr(ip_addr) => {
                        format!("http://{ip_addr}:80/mdns_connect")
                    } // Try default oko camera port
                    MdnsConnectAddress::SocketAddr(socket_addr) => {
                        format!("http://{socket_addr}/mdns_connect")
                    }
                };

                if !camera_form.skip_mdns_connect && state.oko_private_socket_addr.is_some() {
                    let Some(oko_private_socket_addr) = state.oko_private_socket_addr else {
                        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                    };

                    let Ok(resp) = reqwest::Client::new()
                        .post(mdns_connect_url)
                        .form(&[("oko", oko_private_socket_addr.to_string())])
                        .send()
                        .await
                    else {
                        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                    };

                    if resp.status() != StatusCode::OK {
                        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                    }
                } else {
                    debug!(
                        "Skipping mDNS connect, Skip?: {}, Socket Address is Some?: {}",
                        camera_form.skip_mdns_connect,
                        state.oko_private_socket_addr.is_some()
                    );
                }

                let internal_mdns_connect_address = match mdns_connect_address {
                    MdnsConnectAddress::IpAddr(ip_addr) => {
                        // If port is not specified, accept any port
                        ip_addr.to_string() + ":*"
                    }
                    MdnsConnectAddress::SocketAddr(socket_addr) => socket_addr.to_string(),
                };

                let mut camera = Camera {
                    camera_id: Camera::DEFAULT.camera_id,
                    name: camera_form.name,
                    ip_address: Some(internal_mdns_connect_address),
                    last_connected: Camera::DEFAULT.last_connected,
                    is_active: Camera::DEFAULT.is_active,
                };

                if (camera.create_using_self(&state.db_pool).await).is_err() {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }

                let mut camera_setting = CameraSetting {
                    setting_id: CameraSetting::DEFAULT.setting_id,
                    camera_id: camera.camera_id,
                    flashlight_enabled: CameraSetting::DEFAULT.flashlight_enabled,
                    resolution: "SVGA".to_string(),
                    framerate: 5,
                    last_modified: CameraSetting::DEFAULT.last_modified(),
                    modified_by: Some(user.user_id),
                };

                if (camera_setting.create_using_self(&state.db_pool).await).is_err() {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }

                let mut admin_camera_permission = CameraPermission {
                    permission_id: CameraPermission::DEFAULT.permission_id,
                    camera_id: camera.camera_id,
                    user_id: user.user_id,
                    can_view: true,
                    can_control: true,
                };

                if (admin_camera_permission
                    .create_using_self(&state.db_pool)
                    .await)
                    .is_err()
                {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }

                let Ok(all_users) = User::get_all(&state.db_pool).await else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                // TODO: Add test for this
                for user_from_list in all_users {
                    if user_from_list.user_id == admin_camera_permission.user_id {
                        continue;
                    }

                    let mut camera_permission = CameraPermission {
                        permission_id: CameraPermission::DEFAULT.permission_id,
                        camera_id: camera.camera_id,
                        user_id: user_from_list.user_id,
                        can_view: false,
                        can_control: false,
                    };

                    if (camera_permission.create_using_self(&state.db_pool).await).is_err() {
                        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                    }
                }

                if state
                    .api_channel
                    .send(ApiChannelMessage::CameraListChanged(
                        CameraListChange::Added {
                            camera_id: camera.camera_id,
                        },
                    ))
                    .is_err()
                {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }

                Json(camera).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }

    pub async fn camera_restart(
        auth_session: AuthSession,
        Path(camera_id): Path<i64>,
        state: State<Arc<AppState>>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                if user.username != "admin" {
                    return StatusCode::FORBIDDEN.into_response();
                }

                let api_message = ApiChannelMessage::CameraAction {
                    camera_id,
                    message: crate::web::CameraMessage::Restart,
                };

                if state.api_channel.send(api_message).is_err() {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }

                StatusCode::OK.into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct UserForm {
        pub username: String,
        pub password: String,
    }

    pub async fn users(
        auth_session: AuthSession,
        state: State<Arc<AppState>>,
        Form(user_form): Form<UserForm>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                if user.username != "admin" {
                    return StatusCode::FORBIDDEN.into_response();
                }

                if !user_form.username.is_ascii()
                    || !user_form
                        .username
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric())
                {
                    return StatusCode::BAD_REQUEST.into_response();
                }

                let Err(sqlx::Error::RowNotFound) =
                    User::get_using_username(&state.db_pool, &user_form.username).await
                else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                let user_form = if user_form.username == "guest" {
                    UserForm {
                        username: "guest".to_string(),
                        password: "hunter42".to_string(), // TODO: move this
                    }
                } else {
                    user_form
                };

                if user_form.password.len() > 254
                    || user_form.password.is_empty()
                    || user_form.password.contains(char::is_whitespace)
                {
                    return StatusCode::BAD_REQUEST.into_response();
                }

                let Ok(password_hash) =
                    task::spawn_blocking(|| generate_hash(user_form.password)).await
                else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                let mut new_user = User {
                    user_id: User::DEFAULT.user_id,
                    username: user_form.username,
                    password_hash,
                    created_at: User::DEFAULT.created_at(),
                };

                if (new_user.create_using_self(&state.db_pool).await).is_err() {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }

                let Ok(all_cameras) =
                    Camera::list_accessible_to_user(&state.db_pool, user.user_id).await
                else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                for camera in all_cameras {
                    let mut camera_permission = CameraPermission {
                        permission_id: CameraPermission::DEFAULT.permission_id,
                        camera_id: camera.camera_id,
                        user_id: new_user.user_id,
                        can_view: false,
                        can_control: false,
                    };

                    if (camera_permission.create_using_self(&state.db_pool).await).is_err() {
                        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                    }
                }

                Json(new_user).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }
}

// TODO: Don't always return the same error

mod patch {
    use std::sync::Arc;

    use super::{post::UserForm, AuthSession, IntoResponse, StatusCode};
    use crate::{
        web::{AppState, CameraListChange, CameraMessage},
        ApiChannelMessage, CameraPermission, CameraSetting, CameraSettingNoMeta, Model, User,
    };
    use axum::{
        extract::{Path, State},
        Form, Json,
    };
    use password_auth::generate_hash;
    use serde::Deserialize;
    use tracing::warn;

    #[derive(Debug, Clone, Deserialize)]
    pub struct UpdatePermissionForm {
        pub can_view: bool,
        pub can_control: bool,
    }

    pub async fn permissions(
        auth_session: AuthSession,
        state: State<Arc<AppState>>,
        Path(permission_id): Path<i64>,
        Form(permission_form): Form<UpdatePermissionForm>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                if user.username != "admin" {
                    return StatusCode::FORBIDDEN.into_response();
                }

                let Ok(mut permission) =
                    CameraPermission::get_using_id(&state.db_pool, permission_id).await
                else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                permission.can_view = permission_form.can_view;
                permission.can_control = permission_form.can_control;

                if (permission.update_using_self(&state.db_pool).await).is_err() {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }

                if state
                    .api_channel
                    .send(ApiChannelMessage::CameraListChanged(
                        CameraListChange::Updated {
                            camera_id: permission.camera_id,
                        },
                    ))
                    .is_err()
                {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }

                Json(permission).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct UpdateSettingsForm {
        pub flashlight_enabled: bool,
        pub resolution: String,
        pub framerate: i64,
    }

    pub async fn camera_settings(
        auth_session: AuthSession,
        state: State<Arc<AppState>>,
        Path(setting_id): Path<i64>,
        Form(settings_form): Form<UpdateSettingsForm>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                let Ok(mut setting) = CameraSetting::get_using_id(&state.db_pool, setting_id).await
                else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                let Ok(permissions) =
                    CameraPermission::list_for_camera(&state.db_pool, setting.camera_id).await
                else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                if !permissions
                    .iter()
                    .any(|p| (p.user_id == user.user_id) && p.can_control)
                {
                    return StatusCode::FORBIDDEN.into_response();
                }

                // TODO: resolution
                setting.flashlight_enabled = settings_form.flashlight_enabled;

                // ? Maybe allow any framerate/resolution for admin but give warning
                if user.username == "admin" {
                    if (settings_form.framerate < 1) || (settings_form.framerate > 60) {
                        return StatusCode::BAD_REQUEST.into_response();
                    }

                    if !["SVGA", "VGA"].contains(&settings_form.resolution.as_str()) {
                        return StatusCode::BAD_REQUEST.into_response();
                    }

                    setting.resolution = settings_form.resolution;
                    setting.framerate = settings_form.framerate;
                }

                setting.last_modified = CameraSetting::DEFAULT.last_modified();
                setting.modified_by = Some(user.user_id);

                if (setting.update_using_self(&state.db_pool).await).is_err() {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }

                let api_message = ApiChannelMessage::CameraAction {
                    camera_id: setting.camera_id,
                    message: CameraMessage::SettingChanged(CameraSettingNoMeta {
                        flashlight_enabled: setting.flashlight_enabled,
                        resolution: setting.resolution.clone(),
                        framerate: setting.framerate,
                    }),
                };

                if state.api_channel.send(api_message).is_err() {
                    warn!("Failed to send camera_settings update to API channel");
                }

                Json(setting).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }

    pub async fn users(
        auth_session: AuthSession,
        state: State<Arc<AppState>>,
        Path(user_id): Path<i64>,
        Form(user_form): Form<UserForm>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                if user.username != "admin" {
                    return StatusCode::FORBIDDEN.into_response();
                }

                let Ok(mut updated_user) = User::get_using_id(&state.db_pool, user_id).await else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                if !user_form.username.is_ascii()
                    || !user_form
                        .username
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric())
                {
                    return StatusCode::BAD_REQUEST.into_response();
                }

                if updated_user.username != "admin" && updated_user.username != "guest" {
                    updated_user.username = user_form.username;
                }

                if !user_form.password.is_empty() {
                    if user_form.password.len() > 254
                        || user_form.password.contains(char::is_whitespace)
                    {
                        return StatusCode::BAD_REQUEST.into_response();
                    }

                    let Ok(password_hash) =
                        tokio::task::spawn_blocking(|| generate_hash(user_form.password)).await
                    else {
                        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                    };

                    updated_user.password_hash = password_hash;
                }

                if (updated_user.update_using_self(&state.db_pool).await).is_err() {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }

                Json(updated_user).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }
}

mod delete {
    use std::sync::Arc;

    use super::{AuthSession, IntoResponse, StatusCode};
    use crate::{
        web::{AppState, CameraListChange},
        ApiChannelMessage, Camera, Model, User,
    };
    use axum::{
        extract::{Path, State},
        Json,
    };

    pub async fn cameras(
        auth_session: AuthSession,
        state: State<Arc<AppState>>,
        Path(camera_id): Path<i64>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                if user.username != "admin" {
                    return StatusCode::FORBIDDEN.into_response();
                }

                if (Camera::delete_using_id(&state.db_pool, camera_id).await).is_err() {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }

                if state
                    .api_channel
                    .send(ApiChannelMessage::CameraListChanged(
                        CameraListChange::Removed { camera_id },
                    ))
                    .is_err()
                {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }

                Json(camera_id).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }

    pub async fn users(
        auth_session: AuthSession,
        state: State<Arc<AppState>>,
        Path(user_id): Path<i64>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                if user.username != "admin" {
                    return StatusCode::FORBIDDEN.into_response();
                }

                if (User::delete_using_id(&state.db_pool, user_id).await).is_err() {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }

                Json(user_id).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }
}
