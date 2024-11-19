use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Router,
};

use crate::users::AuthSession;

pub fn router() -> Router<()> {
    Router::new()
        .route("/api/", get(self::get::protected))
        .route("/api/cameras", get(self::get::cameras))
        .route("/api/cameras", post(self::post::cameras))
        .route("/api/cameras/:camera_id", delete(self::delete::cameras))
}

mod get {
    use axum::Json;
    use serde::Serialize;

    use crate::{db::Camera, CameraPermissionView, User};

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

                println!("{cameras:?}");

                Json(cameras).into_response()
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
                };

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
                };

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
                };

                Json(camera).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }
}

// TODO: Don't always return the same error

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
                };

                Json(camera_id).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }
}
