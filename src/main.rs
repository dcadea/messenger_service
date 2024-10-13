use axum::http::StatusCode;
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use axum::Router;
use log::error;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

use crate::auth::{cache_user_friends, set_user_context, validate_token};
use crate::state::AppState;

mod auth;
mod chat;
mod error;
mod event;
mod group;
mod integration;
mod markup;
mod message;
mod state;
mod user;

pub(crate) type Result<T> = std::result::Result<T, crate::error::Error>;

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

    let listener = TcpListener::bind("127.0.0.1:8000")
        .await
        .expect("Failed to bind to socket");
    axum::serve(listener, router)
        .await
        .expect("Failed to start server");
}

fn app(app_state: AppState) -> Router {
    let protected_router = Router::new()
        .merge(chat::pages(app_state.clone()))
        .nest(
            "/api",
            Router::new()
                .merge(chat::resources(app_state.clone()))
                .merge(message::resources(app_state.clone()))
                .merge(user::resources(app_state.clone())),
        )
        .route_layer(
            ServiceBuilder::new()
                .layer(from_fn_with_state(app_state.clone(), validate_token))
                .layer(from_fn_with_state(app_state.clone(), set_user_context))
                .layer(from_fn_with_state(app_state.clone(), cache_user_friends)),
        );

    Router::new()
        .merge(auth::endpoints(app_state.clone()))
        .merge(event::endpoints(app_state.clone()))
        .merge(protected_router)
        .route(
            "/health",
            get(|| async { (StatusCode::OK, "I'm good! Hbu?") }),
        )
        .fallback(|| async { (StatusCode::NOT_FOUND, "Why are you here?") })
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
}
