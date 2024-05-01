use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

use crate::chat::repository::ChatRepository;
use crate::chat::service::ChatService;
use crate::integration::client;
use crate::message::repository::MessageRepository;
use crate::message::service::MessageService;
use crate::state::AppState;
use crate::user::repository::UserRepository;
use crate::user::service::UserService;

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

    let database = client::init_mongodb().await;
    let _ = client::init_redis().await;

    let state = AppState {
        message_service: MessageService::new(
            MessageRepository::new(&database),
            client::init_rabbitmq().await,
        ),
        chat_service: ChatService::new(ChatRepository::new(&database)),
        user_service: UserService::new(UserRepository::new(&database)),
    };

    state.clone().message_service.start_purging();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let resources_router = Router::new()
        .merge(chat::api::resources(state.clone()))
        .merge(message::api::resources(state.clone()));

    let router = Router::new()
        .route("/health", get(|| async { () }))
        .nest("/api/v1", resources_router)
        .merge(user::api::auth_router(state.clone()))
        .merge(message::api::ws_router(state.clone()))
        .fallback(|| async { (StatusCode::NOT_FOUND, "Why are you here?") })
        .layer(cors);

    let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
