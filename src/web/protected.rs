use askama::Template;
use axum::{http::StatusCode, response::IntoResponse, routing::get, Router};
use axum_messages::{Message, Messages};

use crate::users::AuthSession;

#[derive(Template)]
#[template(path = "protected.html")]
struct ProtectedTemplate<'a> {
    messages: Vec<Message>,
    username: &'a str,
    user_json: &'a str,
    cameras_json: &'a str,
}

pub fn router() -> Router<()> {
    Router::new().route("/", get(self::get::protected))
}

mod get {
    use crate::db::Camera;

    use super::*;

    pub async fn protected(auth_session: AuthSession, messages: Messages) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                let cameras =
                    Camera::list_accessible_to_user(&auth_session.backend.db, user.user_id)
                    .await
                    .unwrap();

                let safe_user = user.to_redacted_clone();

                ProtectedTemplate {
                    messages: messages.into_iter().collect(),
                    username: &user.username,
                    user_json: &serde_json::to_string(&safe_user).unwrap(),
                    cameras_json: &serde_json::to_string(&cameras).unwrap()
                }
                .into_response()
            }

            None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
