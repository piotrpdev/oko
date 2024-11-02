use axum::{http::StatusCode, response::IntoResponse, routing::get, Router};

use crate::users::AuthSession;

pub fn router() -> Router<()> {
    Router::new().route("/", get(self::get::protected))
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
                    Camera::list_accessible_to_user(&auth_session.backend.db, user.user_id)
                    .await else {
                        return StatusCode::INTERNAL_SERVER_ERROR.into_response()
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
}
