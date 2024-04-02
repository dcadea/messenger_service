#[cfg(test)]
mod test {
    use lapin::options::{BasicGetOptions, BasicPublishOptions, QueueDeclareOptions};
    use lapin::publisher_confirm::Confirmation;
    use lapin::types::FieldTable;
    use lapin::BasicProperties;

    use messenger_api::queue::client::init_rabbitmq;

    #[tokio::test]
    async fn test_queue_message() {
        let conn = init_rabbitmq().await;

        let channel = conn.create_channel().await.unwrap();

        let queue = channel
            .queue_declare(
                "test_queue",
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .unwrap();

        let payload = b"Hello world!";

        let confirm = channel
            .basic_publish(
                "",
                "test_queue",
                BasicPublishOptions::default(),
                payload,
                BasicProperties::default(),
            )
            .await
            .unwrap()
            .await
            .unwrap();

        assert_eq!(confirm, Confirmation::NotRequested);

        let message = channel
            .basic_get("test_queue", BasicGetOptions::default())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(message.data, payload);
    }
}
