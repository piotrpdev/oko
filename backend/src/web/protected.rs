use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Router,
};
use tokio::sync::watch;

use crate::users::AuthSession;
use crate::ApiChannelMessage;

pub fn router(api_channel: watch::Sender<ApiChannelMessage>) -> Router<()> {
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
        .with_state(api_channel)
}

mod get {
    use axum::{body::Body, extract::Path, Json};
    use http::header;
    use serde::Serialize;
    use tokio_util::io::ReaderStream;

    use crate::{
        db::Camera, CameraPermission, CameraPermissionView, CameraSetting, Model, User, Video,
    };

    use super::{AuthSession, IntoResponse, StatusCode};

    #[derive(Serialize)]
    struct ProtectedJson {
        user: User,
        cameras: Vec<CameraPermissionView>,
    }

    pub async fn protected(auth_session: AuthSession) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                // TODO: Handle different error types
                let Ok(cameras) =
                    Camera::list_accessible_to_user(&auth_session.backend.db, user.user_id).await
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

    pub async fn cameras(auth_session: AuthSession) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                let Ok(cameras) =
                    Camera::list_accessible_to_user(&auth_session.backend.db, user.user_id).await
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
        Path(camera_id): Path<i64>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                let Ok(cameras) =
                    Camera::list_accessible_to_user(&auth_session.backend.db, user.user_id).await
                else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                if !cameras.iter().any(|c| c.camera_id == camera_id) {
                    return StatusCode::FORBIDDEN.into_response();
                }

                let Ok(videos) = Video::list_for_camera(&auth_session.backend.db, camera_id).await
                else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                Json(videos).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }

    // Code copied from: https://github.com/tokio-rs/axum/discussions/608
    pub async fn video(auth_session: AuthSession, Path(video_id): Path<i64>) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                let Ok(video) = Video::get_using_id(&auth_session.backend.db, video_id).await
                else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                let Some(video_camera_id) = video.camera_id else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                let Ok(cameras) =
                    Camera::list_accessible_to_user(&auth_session.backend.db, user.user_id).await
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
        Path(camera_id): Path<i64>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                if user.username != "admin" {
                    return StatusCode::FORBIDDEN.into_response();
                }

                let Ok(permissions) = CameraPermission::list_for_camera_with_username(
                    &auth_session.backend.db,
                    camera_id,
                )
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
        Path(camera_id): Path<i64>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(_) => {
                let Ok(settings) =
                    CameraSetting::get_for_camera(&auth_session.backend.db, camera_id).await
                else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                Json(settings).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }
}

// TODO: Don't always return the same error

mod post {
    use super::{AuthSession, IntoResponse, StatusCode};
    use crate::{Camera, CameraPermission, CameraSetting, Model};
    use axum::Form;
    use axum::Json;
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize)]
    pub struct AddCameraForm {
        pub name: String,
        pub address: String,
    }

    // TODO: Create camera permissions for new users

    pub async fn cameras(
        auth_session: AuthSession,
        Form(camera_form): Form<AddCameraForm>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                if user.username != "admin" {
                    return StatusCode::FORBIDDEN.into_response();
                }

                let mut camera = Camera {
                    camera_id: Camera::DEFAULT.camera_id,
                    name: camera_form.name,
                    ip_address: Some(camera_form.address),
                    last_connected: Camera::DEFAULT.last_connected,
                    is_active: Camera::DEFAULT.is_active,
                };

                if (camera.create_using_self(&auth_session.backend.db).await).is_err() {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }

                let mut camera_setting = CameraSetting {
                    setting_id: CameraSetting::DEFAULT.setting_id,
                    camera_id: camera.camera_id,
                    flashlight_enabled: CameraSetting::DEFAULT.flashlight_enabled,
                    resolution: "800x600".to_string(),
                    framerate: 5,
                    last_modified: CameraSetting::DEFAULT.last_modified(),
                    modified_by: Some(user.user_id),
                };

                if (camera_setting
                    .create_using_self(&auth_session.backend.db)
                    .await)
                    .is_err()
                {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }

                let mut camera_permission = CameraPermission {
                    permission_id: CameraPermission::DEFAULT.permission_id,
                    camera_id: camera.camera_id,
                    user_id: user.user_id,
                    can_view: true,
                    can_control: true,
                };

                if (camera_permission
                    .create_using_self(&auth_session.backend.db)
                    .await)
                    .is_err()
                {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }

                Json(camera).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }
}

// TODO: Don't always return the same error

mod patch {
    use super::{AuthSession, IntoResponse, StatusCode};
    use crate::{
        web::CameraMessage, ApiChannelMessage, CameraPermission, CameraSetting,
        CameraSettingNoMeta, Model,
    };
    use axum::{
        extract::{Path, State},
        Form, Json,
    };
    use serde::Deserialize;
    use tokio::sync::watch;
    use tracing::warn;

    #[derive(Debug, Clone, Deserialize)]
    pub struct UpdatePermissionForm {
        pub can_view: bool,
        pub can_control: bool,
    }

    pub async fn permissions(
        auth_session: AuthSession,
        Path(permission_id): Path<i64>,
        Form(permission_form): Form<UpdatePermissionForm>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                if user.username != "admin" {
                    return StatusCode::FORBIDDEN.into_response();
                }

                let Ok(mut permission) =
                    CameraPermission::get_using_id(&auth_session.backend.db, permission_id).await
                else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                permission.can_view = permission_form.can_view;
                permission.can_control = permission_form.can_control;

                if (permission.update_using_self(&auth_session.backend.db).await).is_err() {
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
        // pub resolution: String,
        pub framerate: i64,
    }

    pub async fn camera_settings(
        auth_session: AuthSession,
        state: State<watch::Sender<ApiChannelMessage>>,
        Path(setting_id): Path<i64>,
        Form(settings_form): Form<UpdateSettingsForm>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                let Ok(mut setting) =
                    CameraSetting::get_using_id(&auth_session.backend.db, setting_id).await
                else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                let Ok(permissions) =
                    CameraPermission::list_for_camera(&auth_session.backend.db, setting.camera_id)
                        .await
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

                    // setting.resolution = settings_form.resolution;
                    setting.framerate = settings_form.framerate;
                }

                setting.last_modified = CameraSetting::DEFAULT.last_modified();
                setting.modified_by = Some(user.user_id);

                if (setting.update_using_self(&auth_session.backend.db).await).is_err() {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }

                let api_message = ApiChannelMessage::CameraRelated {
                    camera_id: setting.camera_id,
                    message: CameraMessage::SettingChanged(CameraSettingNoMeta {
                        flashlight_enabled: setting.flashlight_enabled,
                        resolution: setting.resolution.clone(),
                        framerate: setting.framerate,
                    }),
                };

                if state.send(api_message).is_err() {
                    warn!("Failed to send camera_settings update to API channel");
                }

                Json(setting).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }
}

mod delete {
    use super::{AuthSession, IntoResponse, StatusCode};
    use crate::{Camera, Model};
    use axum::{extract::Path, Json};

    pub async fn cameras(
        auth_session: AuthSession,
        Path(camera_id): Path<i64>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                if user.username != "admin" {
                    return StatusCode::FORBIDDEN.into_response();
                }

                if (Camera::delete_using_id(&auth_session.backend.db, camera_id).await).is_err() {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }

                Json(camera_id).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }
}
