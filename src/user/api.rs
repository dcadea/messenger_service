use axum::extract::{Query, State};
use axum::response::Redirect;
use axum::routing::get;
use axum::{Json, Router};
use openid::Bearer;

use crate::result::Result;
use crate::state::AppState;
use crate::user::model::CallbackParams;

pub fn auth_router<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/login", get(login_handler))
        .route("/callback", get(callback_handler))
        .with_state(state)
}

async fn login_handler(state: State<AppState>) -> Redirect {
    Redirect::to(state.user_service.authorize_url().as_str())
}

async fn callback_handler(
    params: Query<CallbackParams>,
    state: State<AppState>,
) -> Result<Json<Bearer>> {
    state
        .user_service
        .request_token(&params.code)
        .await
        .map(Json)
}
