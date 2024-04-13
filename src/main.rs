use std::convert::Infallible;
use std::sync::Arc;

use warp::Filter;

use handler::health_handler;
use message::handler::{messages_handler, ws_handler};
use message::service::MessageService;
use user::handler::login_handler;

use crate::integration::client::ClientFactory;
use crate::message::repository::MessageRepository;
use crate::message::service::start_purging;
use crate::user::handler::register_handler;
use crate::user::repository::UserRepository;

mod error;
mod handler;
mod integration;
mod message;
mod user;

#[tokio::main]
async fn main() {
    env_logger::init();

    let database = ClientFactory::init_mongodb().await;
    let user_repository = UserRepository::new(&database);
    let message_repository = MessageRepository::new(&database);

    let rabbitmq_client = ClientFactory::init_rabbitmq().await;
    let message_service = MessageService::new(rabbitmq_client.clone());

    start_purging(message_service.clone(), message_repository.clone());

    let health_route = warp::path!("health").and_then(health_handler);

    let messages = warp::path!("messages")
        .and(warp::body::json())
        .and(with_message_service(message_service.clone()))
        .and_then(messages_handler);

    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(warp::path::param())
        .and(with_user_repository(user_repository.clone()))
        .and(with_message_service(message_service.clone()))
        .and_then(ws_handler);

    let login_route = warp::path("login")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_user_repository(user_repository.clone()))
        .and_then(login_handler);

    let register_route = warp::path("register")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_user_repository(user_repository.clone()))
        .and_then(register_handler);

    let routes = health_route
        .or(ws_route)
        .or(messages)
        .or(login_route)
        .or(register_route)
        .with(
            warp::cors()
                .allow_any_origin()
                .allow_origins(vec!["http://localhost:4200"])
                .allow_headers(vec![
                    "Content-Type",
                    "Access-Control-Request-Method",
                    "Access-Control-Request-Headers",
                ])
                .allow_methods(vec!["GET", "POST", "DELETE", "PUT", "OPTIONS"]),
        );

    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}

fn with_user_repository(
    repository: Arc<UserRepository>,
) -> impl Filter<Extract = (Arc<UserRepository>,), Error = Infallible> + Clone {
    warp::any().map(move || repository.clone())
}

fn with_message_service(
    service: Arc<MessageService>,
) -> impl Filter<Extract = (Arc<MessageService>,), Error = Infallible> + Clone {
    warp::any().map(move || service.clone())
}
