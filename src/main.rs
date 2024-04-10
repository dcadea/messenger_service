use std::convert::Infallible;
use std::sync::Arc;

use tokio::sync::Mutex;
use warp::Filter;
use handler::health_handler;
use message::handler::{messages_handler, ws_handler};

use crate::integration::client::ClientFactory;
use crate::message::repository::MessageRepository;
use message::service::MessageService;
use user::handler::login_handler;

use crate::user::repository::UserRepository;

mod error;
mod handler;
mod integration;
mod message;
mod user;

#[tokio::main]
async fn main() {
    env_logger::init();

    // TODO
    let _ = ClientFactory::init_redis().await;

    let database = ClientFactory::init_mongodb().await;
    let user_repository = Arc::new(UserRepository::new(&database));
    let message_repository = Arc::new(MessageRepository::new(&database));

    let rabbitmq_client = Arc::new(Mutex::new(ClientFactory::init_rabbitmq().await));
    let message_service = Arc::new(MessageService::new(rabbitmq_client, message_repository));

    let health_route = warp::path!("health").and_then(health_handler);

    let messages = warp::path!("messages")
        .and(warp::body::json())
        .and(with_message_service(Arc::clone(&message_service)))
        .and_then(messages_handler);

    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(warp::path::param())
        .and(with_user_repository(Arc::clone(&user_repository)))
        .and(with_message_service(Arc::clone(&message_service)))
        .and_then(ws_handler);

    let login_route = warp::path("login")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_user_repository(user_repository))
        .and_then(login_handler);

    let routes = health_route.or(ws_route).or(messages).or(login_route).with(
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

// TODO
// fn with_message_repository(repository: MessageRepository) -> impl Filter<Extract=(MessageRepository, ), Error=Infallible> + Clone {
//     warp::any().map(move || repository.clone())
// }
//
// fn with_redis_client(redis_client: redis::Client) -> impl Filter<Extract=(redis::Client, ), Error=Infallible> + Clone {
//     warp::any().map(move || redis_client.clone())
// }
