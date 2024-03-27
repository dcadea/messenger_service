use lapin::{Connection, ConnectionProperties};

pub async fn init_rabbitmq() -> Connection {
    let addr = std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://127.0.0.1:5672/%2f".into());

    let conn = Connection::connect(
        &addr,
        ConnectionProperties::default(),
    ).await;

    match conn {
        Ok(conn) => conn,
        Err(e) => panic!("Error connecting to RabbitMQ: {}", e),
    }
}
