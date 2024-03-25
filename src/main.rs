use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

use tokio::sync::RwLock;
use warp::{Filter, Rejection};

use crate::user::repository::UserRepository;
use crate::ws::model::Clients;

mod user;
mod error;
mod db;
mod ws;
mod handler;

type Result<T> = std::result::Result<T, Rejection>;

#[tokio::main]
async fn main() {
    env_logger::init();

    let clients: Clients = Arc::new(RwLock::new(HashMap::new()));

    let database = db::client::init_mongodb().await;
    let user_repository = UserRepository::new(database);

    let health_route = warp::path!("health").and_then(handler::health_handler);

    let register = warp::path("register");
    let register_routes = register
        .and(warp::post())
        .and(warp::body::json())
        .and(with_clients(clients.clone()))
        .and_then(ws::handler::register_handler)
        .or(register
            .and(warp::delete())
            .and(warp::path::param())
            .and(with_clients(clients.clone()))
            .and_then(ws::handler::unregister_handler));

    let publish = warp::path!("publish")
        .and(warp::body::json())
        .and(with_clients(clients.clone()))
        .and_then(ws::handler::publish_handler);

    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(warp::path::param())
        .and(with_clients(clients.clone()))
        .and_then(ws::handler::ws_handler);

    let login_route = warp::path("login")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_user_repository(user_repository.clone()))
        .and_then(user::handler::login_handler);

    let routes = health_route
        .or(register_routes)
        .or(ws_route)
        .or(publish)
        .or(login_route)
        .with(warp::cors()
            .allow_any_origin()
            .allow_origins(vec!["http://localhost:4200"])
            .allow_headers(vec![
                "Content-Type",
                "Access-Control-Request-Method",
                "Access-Control-Request-Headers",
            ])
            .allow_methods(vec!["GET", "POST", "DELETE", "PUT", "OPTIONS"])
        );


    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}

fn with_clients(clients: Clients) -> impl Filter<Extract=(Clients, ), Error=Infallible> + Clone {
    warp::any().map(move || clients.clone())
}

fn with_user_repository(repository: UserRepository) -> impl Filter<Extract=(UserRepository, ), Error=Infallible> + Clone {
    warp::any().map(move || repository.clone())
}
