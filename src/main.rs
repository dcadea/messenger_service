use std::convert::Infallible;
use std::sync::Arc;

use tokio::sync::Mutex;
use warp::{Filter, Rejection};

use crate::user::repository::UserRepository;

mod cache;
mod db;
mod error;
mod handler;
mod message;
mod queue;
mod user;
mod ws;

type Result<T> = std::result::Result<T, Rejection>;

#[tokio::main]
async fn main() {
    env_logger::init();

    let redis_con = Arc::new(Mutex::new(cache::client::init_redis().await));
    let ws_client_service = Arc::new(ws::service::init_ws_client_service(redis_con));

    let database = db::client::init_mongodb().await;
    let user_repository = Arc::new(UserRepository::new(database));

    // TODO
    // let _ = cache::client::init_redis().await;

    let health_route = warp::path!("health").and_then(handler::health_handler);

    let register = warp::path("register");
    let register_routes = register
        .and(warp::post())
        .and(warp::path::param())
        .and(warp::body::json())
        .and(with_ws_client_service(Arc::clone(&ws_client_service)))
        .and_then(ws::handler::register_handler)
        .or(register
            .and(warp::delete())
            .and(warp::path::param())
            .and(with_ws_client_service(Arc::clone(&ws_client_service)))
            .and_then(ws::handler::unregister_handler));

    let publish = warp::path!("publish")
        .and(warp::body::json())
        .and(with_ws_client_service(Arc::clone(&ws_client_service)))
        .and_then(ws::handler::publish_handler);

    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(warp::path::param())
        .and(with_ws_client_service(Arc::clone(&ws_client_service)))
        .and_then(ws::handler::ws_handler);

    let login_route = warp::path("login")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_user_repository(user_repository))
        .and_then(user::handler::login_handler);

    let routes = health_route
        .or(register_routes)
        .or(ws_route)
        .or(publish)
        .or(login_route)
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

fn with_ws_client_service(
    service: Arc<ws::service::WsClientService>,
) -> impl Filter<Extract = (Arc<ws::service::WsClientService>,), Error = Infallible> + Clone {
    warp::any().map(move || service.clone())
}

fn with_user_repository(
    repository: Arc<UserRepository>,
) -> impl Filter<Extract = (Arc<UserRepository>,), Error = Infallible> + Clone {
    warp::any().map(move || repository.clone())
}

// TODO
// fn with_message_repository(repository: MessageRepository) -> impl Filter<Extract=(MessageRepository, ), Error=Infallible> + Clone {
//     warp::any().map(move || repository.clone())
// }
//
// fn with_redis_client(redis_client: redis::Client) -> impl Filter<Extract=(redis::Client, ), Error=Infallible> + Clone {
//     warp::any().map(move || redis_client.clone())
// }
