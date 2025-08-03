use crate::markup::wrap_in_base;
use auth::middleware::{authorize, validate_sid};
use axum::Router;
use axum::http::StatusCode;
use axum::middleware::{from_fn_with_state, map_response};
use axum::routing::get;
use integration::Env;
use log::error;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

mod auth;
mod contact;
mod error;
mod event;
mod handler;
mod integration;
mod markup;
mod message;
mod schema;
mod settings;
mod state;
mod talk;
mod user;

pub type Result<T> = std::result::Result<T, crate::error::Error>;

#[tokio::main]
async fn main() {
    let config = integration::Config::default();
    let app_state = match AppState::init(config.clone()).await {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to initialize app state: {e:?}");
            return;
        }
    };

    if let Err(e) = {
        let env = config.env();
        let addr = env.addr();
        let router = app(&app_state, env);

        match env.ssl_config() {
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
    } {
        panic!("Failed to start server: {e:?}")
    }
}

fn app(s: &AppState, env: &Env) -> Router {
    let protected_router = Router::new()
        .route("/", get(handler::home))
        .merge(talk::pages(s.clone()))
        .merge(event::api(s.clone()))
        .nest(
            "/tabs",
            Router::new()
                .route("/chats", get(handler::chats_tab))
                .route("/groups", get(handler::groups_tab))
                .route("/contacts", get(handler::contacts_tab))
                .route("/settings", get(handler::settings_tab)),
        )
        .nest(
            "/api",
            Router::new()
                .merge(contact::api(s.clone()))
                .merge(message::api(s.clone()))
                .merge(talk::api(s.clone()))
                .merge(user::api(s.clone())),
        )
        .nest(
            "/templates",
            Router::new()
                .merge(message::templates(s.clone()))
                .merge(talk::templates(s.clone())),
        )
        .route_layer(
            ServiceBuilder::new()
                .layer(from_fn_with_state(s.clone(), validate_sid))
                .layer(from_fn_with_state(s.clone(), authorize)),
        )
        .with_state(s.clone());

    Router::new()
        .nest_service("/static", ServeDir::new("static"))
        .merge(auth::pages(s.clone()))
        .merge(auth::api(s.clone()))
        .merge(protected_router)
        .route(
            "/health",
            get(|| async { (StatusCode::OK, "I'm good! Hbu?") }),
        )
        .fallback(|| async { (StatusCode::NOT_FOUND, "Why are you here?") })
        .route_layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(
                    CorsLayer::new()
                        .allow_origin(env.allow_origin())
                        .allow_methods(env.allow_methods())
                        .allow_headers(env.allow_headers()),
                )
                .layer(map_response(async |r| wrap_in_base(r))),
        )
}
