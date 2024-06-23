use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::get;
use axum::Router;

use crate::result::Result;
use crate::state::AppState;
use crate::user::service::UserService;

use super::handle_socket;
use super::service::EventService;

pub fn ws_router<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(event_service): State<EventService>,
    State(user_service): State<UserService>,
) -> Result<Response> {
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, event_service, user_service)))
}
