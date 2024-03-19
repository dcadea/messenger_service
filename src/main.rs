use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

use tokio::sync::RwLock;
use warp::{Filter, Rejection};

use repository::UserRepository;

mod handler;
mod ws;
mod repository;
mod models;

const MONGO_URI: &str = "mongodb://root:example@localhost:27017";
const MONGO_DB: &str = "messenger";

type Result<T> = std::result::Result<T, Rejection>;
type Clients = Arc<RwLock<HashMap<String, models::Client>>>;

#[tokio::main]
async fn main() {
    let clients: Clients = Arc::new(RwLock::new(HashMap::new()));

    let database = mongodb::Client::with_uri_str(MONGO_URI).await.unwrap().database(MONGO_DB);
    let user_repository = UserRepository::new(database);

    let health_route = warp::path!("health").and_then(handler::health_handler);

    let register = warp::path("register");
    let register_routes = register
        .and(warp::post())
        .and(warp::body::json())
        .and(with_clients(clients.clone()))
        .and_then(handler::register_handler)
        .or(register
            .and(warp::delete())
            .and(warp::path::param())
            .and(with_clients(clients.clone()))
            .and_then(handler::unregister_handler));

    let publish = warp::path!("publish")
        .and(warp::body::json())
        .and(with_clients(clients.clone()))
        .and_then(handler::publish_handler);

    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(warp::path::param())
        .and(with_clients(clients.clone()))
        .and_then(handler::ws_handler);

    let login_route = warp::path("login")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_user_repository(user_repository.clone()))
        .and_then(handler::login_handler);

    let routes = health_route
        .or(register_routes)
        .or(ws_route)
        .or(publish)
        .or(login_route)
        .with(warp::cors().allow_any_origin());

    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}

fn with_clients(clients: Clients) -> impl Filter<Extract=(Clients, ), Error=Infallible> + Clone {
    warp::any().map(move || clients.clone())
}

fn with_user_repository(repository: UserRepository) -> impl Filter<Extract=(UserRepository, ), Error=Infallible> + Clone {
    warp::any().map(move || repository.clone())
}
