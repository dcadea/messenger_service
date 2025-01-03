use auth::middleware::{authorize, validate_sid};
use axum::http::StatusCode;
use axum::middleware::{from_fn, from_fn_with_state, map_response};
use axum::routing::get;
use axum::Router;
use log::error;
use messenger_service::markup::wrap_in_base;
use messenger_service::middleware::attach_request_id;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use user::middleware::cache_user_friends;

use crate::state::AppState;

mod auth;
mod chat;
mod error;
mod event;
mod integration;
mod message;
mod state;
mod user;

pub type Result<T> = std::result::Result<T, crate::error::Error>;

#[tokio::main]
async fn main() {
    let config = integration::Config::default();
    let addr = match config.env {
        integration::Environment::Local => "127.0.0.1:8000",
        integration::Environment::Docker => "0.0.0.0:8000",
    };
    let app_state = match AppState::init(config).await {
        Ok(state) => state,
        Err(e) => {
            error!("Failed to initialize app state: {:?}", e);
            return;
        }
    };

    let router = app(app_state.clone());

    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind to socket");
    axum::serve(listener, router)
        .await
        .expect("Failed to start server");
}

fn app(app_state: AppState) -> Router {
    let protected_router = Router::new()
        .merge(chat::pages(app_state.clone()))
        .merge(event::api(app_state.clone()))
        .nest(
            "/api",
            Router::new()
                .merge(chat::api(app_state.clone()))
                .merge(message::api(app_state.clone()))
                .merge(user::api(app_state.clone())),
        )
        .route_layer(
            ServiceBuilder::new()
                .layer(from_fn_with_state(app_state.clone(), validate_sid))
                .layer(from_fn_with_state(app_state.clone(), authorize))
                .layer(from_fn_with_state(app_state.clone(), cache_user_friends)),
        );

    Router::new()
        .nest_service("/assets", ServeDir::new("assets"))
        .merge(auth::pages(app_state.clone()))
        .merge(auth::api(app_state.clone()))
        .merge(protected_router)
        .route(
            "/health",
            get(|| async { (StatusCode::OK, "I'm good! Hbu?") }),
        )
        .fallback(|| async { (StatusCode::NOT_FOUND, "Why are you here?") })
        .route_layer(
            ServiceBuilder::new()
                .layer(from_fn(attach_request_id))
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any),
                )
                .layer(map_response(wrap_in_base)),
        )
}
