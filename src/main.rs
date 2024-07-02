use axum::http::StatusCode;
use axum::middleware::from_fn_with_state;
use axum::Router;
use axum::routing::get;
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
                .layer(from_fn_with_state(app_state.clone(), set_user_context)),
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

#[cfg(test)]
mod tests {
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::{app, integration};
    use crate::state::AppState;

    #[tokio::test]
    async fn test_health() {
        let test_config = integration::Config::test();

        let app_state = AppState::init(test_config.await).await.unwrap();
        let router = app(app_state);

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        //TODO: assert_eq!(response.body(), "I'm good! Hbu?");
    }
}
