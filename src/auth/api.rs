use axum::http::StatusCode;
use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
    routing::get,
    Router,
};
use axum_extra::extract::Query;

use crate::result::Result;
use crate::state::AppState;

use super::{model::CallbackParams, service::AuthService};

pub fn endpoints<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/login", get(login_handler))
        .route(
            "/logout",
            get(|| async { (StatusCode::OK, "Mocking logout :)") }),
        )
        .route("/callback", get(callback_handler))
        .with_state(state)
}

async fn login_handler(auth_service: State<AuthService>) -> Result<impl IntoResponse> {
    Ok(Redirect::to(&auth_service.authorize().await))
}

async fn callback_handler(
    params: Query<CallbackParams>,
    auth_service: State<AuthService>,
) -> Result<impl IntoResponse> {
    let token = auth_service.exchange_code(&params.code).await?;
    Ok((StatusCode::OK, token)) // TODO: set in session storage
}
