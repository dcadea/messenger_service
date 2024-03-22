use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use mongodb::{Client, Database};
use mongodb::options::ClientOptions;

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
    env_logger::init();

    let clients: Clients = Arc::new(RwLock::new(HashMap::new()));

    let database = init_mongodb().await;
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

async fn init_mongodb() -> Database {
    let mut mongo_client_options = ClientOptions::parse(MONGO_URI).await.unwrap();
    mongo_client_options.connect_timeout = Some(Duration::from_secs(5));
    mongo_client_options.server_selection_timeout = Some(Duration::from_secs(2));
    let client = Client::with_options(mongo_client_options).unwrap();
    let database = client.database(MONGO_DB);
    database
}
