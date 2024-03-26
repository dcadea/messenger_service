#[cfg(test)]
mod tests {
    use mongodb::Database;

    use messenger_api::message::model::Message;
    use messenger_api::message::repository::MessageRepository;

    const MONGO_URI: &str = "mongodb://root:example@localhost:27017";


    #[tokio::test]
    async fn test_insert_message() {
        let database: Database = mongodb::Client::with_uri_str(MONGO_URI).await.unwrap().database("messenger");
        let repository: MessageRepository = MessageRepository::new(database);

        let text = "Hello, world!";
        let sender = "me";
        let recipient = "you";
        let message = Message::new(sender, recipient, text, 1234567890);
        let result = repository.insert(message).await;

        match result {
            Ok(_) => {
                let messages = repository.find_by_recipient(recipient).await.unwrap();
                assert!(messages.iter().any(|m| m.text() == text), "Expected to find a message with content '{}'", text);
            }
            Err(err) => panic!("Failed to insert message: {}", err),
        }

        repository.delete_by_sender(sender).await.unwrap();
    }
}
