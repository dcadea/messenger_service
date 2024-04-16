use axum::routing::get;
use axum::Router;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

use crate::integration::client::ClientFactory;
use crate::message::repository::MessageRepository;
use crate::message::service::{start_purging, MessageService};
use crate::state::AppState;
use crate::user::repository::UserRepository;

mod error;
mod integration;
mod message;
mod state;
mod user;

#[tokio::main]
async fn main() {
    env_logger::init();

    let database = ClientFactory::init_mongodb().await;

    let rabbitmq_client = ClientFactory::init_rabbitmq().await;

    let state = AppState {
        message_service: MessageService::new(rabbitmq_client.clone()),
        user_repository: UserRepository::new(&database),
        message_repository: MessageRepository::new(&database),
    };

    start_purging(state.clone());

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let router = Router::new()
        .route("/health", get(|| async { () }))
        .merge(message::api::router(state.clone()))
        .merge(user::api::router(state.clone()))
        .layer(cors);

    let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
