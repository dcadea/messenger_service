use axum::http::StatusCode;
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use axum::Router;
use log::error;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

use auth::{set_user_context, validate_token};
use state::AppState;

mod auth;
mod chat;
mod error;
mod event;
mod group;
mod integration;
mod message;
mod result;
mod state;
mod user;
mod util;

#[tokio::main]
async fn main() {
    let app_state = match AppState::init().await {
        Ok(state) => state,
        Err(e) => {
            error!("Failed to initialize app state: {:?}", e);
            return;
        }
    };

    let api_router = Router::new()
        .merge(chat::api::resources(app_state.clone()))
        .merge(message::api::resources(app_state.clone()))
        .merge(user::api::resources(app_state.clone()))
        .route_layer(
            ServiceBuilder::new()
                .layer(from_fn_with_state(app_state.clone(), validate_token))
                .layer(from_fn_with_state(app_state.clone(), set_user_context)),
        );

    let router = Router::new()
        .route("/health", get(|| async { () }))
        .merge(event::api::ws_router(app_state.clone()))
        .nest("/api/v1", api_router)
        .fallback(|| async { (StatusCode::NOT_FOUND, "Why are you here?") })
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    let socket = app_state.clone().config.socket;
    let listener = TcpListener::bind(socket).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
