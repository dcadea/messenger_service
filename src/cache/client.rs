use redis::{Client, Connection};

pub async fn init_redis() -> Connection {
    let host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = std::env::var("REDIS_PORT").unwrap_or_else(|_| "6379".into());

    match Client::open(format!("redis://{}:{}", host, port)) {
        Ok(client) => match client.get_connection_with_timeout(std::time::Duration::from_secs(2)) {
            Ok(connection) => connection,
            Err(e) => panic!("Error connecting to redis: {}", e),
        },
        Err(e) => panic!("Error creating redis client: {}", e),
    }
}
