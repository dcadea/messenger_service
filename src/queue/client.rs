use redis::{Client, Connection};

pub async fn init_redis() -> Connection {
    match Client::open("redis://localhost:6379") {
        Ok(client) => {
            match client.get_connection_with_timeout(std::time::Duration::from_secs(2)) {
                Ok(connection) => connection,
                Err(e) => panic!("Error connecting to redis: {}", e),
            }
        }
        Err(e) => panic!("Error creating redis client: {}", e)
    }
}
