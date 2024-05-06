use crate::auth::validate_token;
use axum::http::StatusCode;
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use axum::{Extension, Router};
use log::error;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

use crate::state::{AppState, AuthState};

mod auth;
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
    let config = integration::Config::default();

    let auth_state = match AuthState::init(&config).await {
        Ok(state) => state,
        Err(e) => {
            error!("Failed to initialize auth state: {:?}", e);
            return;
        }
    };

    let app_state = match AppState::init(&config).await {
        Ok(state) => state,
        Err(e) => {
            error!("Failed to initialize app state: {:?}", e);
            return;
        }
    };

    app_state.clone().message_service.start_purging();

    let resources_router = Router::new()
        .merge(chat::api::resources(app_state.clone()))
        .merge(message::api::resources(app_state.clone()))
        .route_layer(from_fn_with_state(auth_state.clone(), validate_token));

    let router = Router::new()
        .route("/health", get(|| async { () }))
        .nest("/api/v1", resources_router)
        .merge(message::api::ws_router(app_state.clone()))
        .fallback(|| async { (StatusCode::NOT_FOUND, "Why are you here?") })
        .layer(Extension(config))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
