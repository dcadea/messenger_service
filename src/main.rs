use auth::middleware::{authorize, validate_sid};
use axum::Router;
use axum::http::StatusCode;
use axum::middleware::{from_fn, from_fn_with_state, map_response};
use axum::routing::get;
use integration::Environment;
use log::error;
use messenger_service::markup::wrap_in_base;
use messenger_service::middleware::attach_request_id;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

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
    let app_state = match AppState::init(config.clone()).await {
        Ok(state) => state,
        Err(e) => {
            error!("Failed to initialize app state: {:?}", e);
            return;
        }
    };
    let router = app(app_state.clone(), &config.env);

    let addr = config.env.addr();
    let ssl_config = config.env.ssl_config();

    match ssl_config {
        Some(ssl_config) => {
            axum_server::bind_openssl(addr, ssl_config)
                .serve(router.into_make_service())
                .await
        }
        None => {
            axum_server::bind(addr)
                .serve(router.into_make_service())
                .await
        }
    }
    .expect("Failed to start server")
}

fn app(app_state: AppState, env: &Environment) -> Router {
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
        .nest(
            "/templates",
            Router::new().merge(message::templates(app_state.clone())),
        )
        .route_layer(
            ServiceBuilder::new()
                .layer(from_fn_with_state(app_state.clone(), validate_sid))
                .layer(from_fn_with_state(app_state.clone(), authorize)),
        );

    Router::new()
        .nest_service("/static", ServeDir::new("static"))
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
                        .allow_origin(env.allow_origin())
                        .allow_methods(env.allow_methods())
                        .allow_headers(env.allow_headers()),
                )
                .layer(map_response(wrap_in_base)),
        )
}
