use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use log::error;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

use crate::state::AppState;

mod chat;
mod error;
mod integration;
mod message;
mod result;
mod state;
mod user;

#[tokio::main]
async fn main() {
    env_logger::init();

    let state = match AppState::init().await {
        Ok(state) => state,
        Err(e) => {
            error!("Failed to initialize application: {:?}", e);
            return;
        }
    };

    state.clone().message_service.start_purging();

    let resources_router = Router::new()
        .merge(chat::api::resources(state.clone()))
        .merge(message::api::resources(state.clone()));

    let router = Router::new()
        .route("/health", get(|| async { () }))
        .nest("/api/v1", resources_router)
        .merge(user::api::auth_router(state.clone()))
        .merge(message::api::ws_router(state.clone()))
        .fallback(|| async { (StatusCode::NOT_FOUND, "Why are you here?") })
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
