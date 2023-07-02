use axum::{routing::get, Json, Router};

use crate::{
    domain::auth::model::User,
    shared::{AppError, AppState},
};

pub mod email;
pub mod google;

#[allow(unused)]
pub fn gen_service(state: AppState) -> Router {
    Router::new()
        .route("/me", get(me))
        .nest_service("/email", email::gen_service(state.clone()))
        .nest_service("/google", google::gen_service(state.clone()))
        .with_state(state)
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    responses(
        (status = 200)
    ),
    security(
        ("bearer_token" = [])
    )
)]
pub async fn me(user: User) -> Result<Json<User>, AppError> {
    Ok(Json(user))
}
