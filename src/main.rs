use axum::http::StatusCode;
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use axum::Router;
use log::error;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

use crate::auth::cache_user_friends;
use auth::{set_user_context, validate_token};
use state::AppState;

mod auth;
mod chat;
mod error;
mod event;
mod group;
mod integration;
mod message;
mod model;
mod result;
mod state;
mod user;
mod util;

#[tokio::main]
async fn main() {
    let config = integration::Config::default();
    let app_state = match AppState::init(config).await {
        Ok(state) => state,
        Err(e) => {
            error!("Failed to initialize app state: {:?}", e);
            return;
        }
    };

    let router = app(app_state.clone());

    let socket = app_state.config.socket;
    let listener = TcpListener::bind(socket)
        .await
        .expect("Failed to bind to socket");
    axum::serve(listener, router)
        .await
        .expect("Failed to start server");
}

fn app(app_state: AppState) -> Router {
    let api_router = Router::new()
        .merge(chat::api::resources(app_state.clone()))
        .merge(message::api::resources(app_state.clone()))
        .merge(user::api::resources(app_state.clone()))
        .route_layer(
            ServiceBuilder::new()
                .layer(from_fn_with_state(app_state.clone(), validate_token))
                .layer(from_fn_with_state(app_state.clone(), set_user_context))
                .layer(from_fn_with_state(app_state.clone(), cache_user_friends)),
        );

    Router::new()
        .route(
            "/health",
            get(|| async { (StatusCode::OK, "I'm good! Hbu?") }),
        )
        .merge(event::api::ws_router(app_state.clone()))
        .nest("/api/v1", api_router)
        .fallback(|| async { (StatusCode::NOT_FOUND, "Why are you here?") })
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
}
