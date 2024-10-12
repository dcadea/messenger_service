use auth::set_test_user_context;
use axum::http::StatusCode;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::routing::get;
use axum::{Extension, Router};
use futures::FutureExt;
use log::error;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use user::model::UserInfo;

use crate::auth::{cache_user_friends, set_user_context, validate_token};
use crate::markup::Wrappable;
use crate::state::AppState;

mod auth;
mod chat;
mod error;
mod event;
mod group;
mod integration;
mod markup;
mod message;
mod model;
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

    let socket = app_state.config.socket;
    let listener = TcpListener::bind(socket)
        .await
        .expect("Failed to bind to socket");
    axum::serve(listener, router)
        .await
        .expect("Failed to start server");
}

async fn root(logged_user: Extension<UserInfo>) -> Wrappable {
    chat::markup::all_chats(logged_user)
        .await
        .map(|r| markup::Wrappable(r))
        .expect("Failed to render root")
}

fn app(app_state: AppState) -> Router {
    let pages_router = Router::new()
        .route("/", get(root).route_layer(from_fn(markup::wrap_in_base)))
        .merge(chat::markup::pages(app_state.clone()))
        .layer(from_fn_with_state(app_state.clone(), set_test_user_context));

    let resources_router = Router::new()
        .merge(chat::markup::resources(app_state.clone()))
        .merge(message::markup::resources(app_state.clone()))
        .merge(user::markup::resources(app_state.clone()))
        .route_layer(
            ServiceBuilder::new()
                // .layer(from_fn_with_state(app_state.clone(), validate_token))
                .layer(from_fn_with_state(app_state.clone(), set_test_user_context))
                .layer(from_fn_with_state(app_state.clone(), cache_user_friends)),
        );

    Router::new()
        .merge(auth::api::endpoints(app_state.clone()))
        .merge(pages_router)
        .nest("/api", resources_router)
        .route(
            "/health",
            get(|| async { (StatusCode::OK, "I'm good! Hbu?") }),
        )
        .merge(event::api::ws_router(app_state.clone()))
        .fallback(|| async { (StatusCode::NOT_FOUND, "Why are you here?") })
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
}
