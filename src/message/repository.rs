use futures::stream::TryStreamExt;
use log::{debug, error};
use mongodb::{bson, Database};
use mongodb::bson::{doc, Document};
use mongodb::error::Error;
use mongodb::results::InsertOneResult;

use crate::message::model::Message;

#[derive(Clone)]
pub struct MessageRepository {
    collection: mongodb::Collection<Document>,
}

impl MessageRepository {
    pub fn new(database: Database) -> Self {
        let collection = database.collection("messages");
        Self { collection }
    }

    pub async fn insert(&self, message: Message) -> Result<InsertOneResult, Error> {
        debug!("Inserting message: {:?}", message);
        let document = bson::to_document(&message).unwrap();
        match self.collection.insert_one(document, None).await {
            Ok(result) => Ok(result),
            Err(e) => {
                error!("Failed to insert message from: {} to: {} on {}",
                    message.sender(), message.recipient(), message.timestamp());
                Err(e)
            }
        }
    }

    pub async fn find_by_recipient(&self, recipient: &str) -> Result<Vec<Message>, Error> {
        debug!("Finding messages by recipient: {}", recipient);
        let filter = doc! { "recipient": recipient };

        let mut cursor = self.collection.find(filter, None).await?;

        let mut messages = Vec::new();

        while let Some(doc) = cursor.try_next().await? {
            let message = bson::from_document(doc).unwrap();
            messages.push(message);
        }

        Ok(messages)
    }

    pub async fn delete_by_sender(&self, sender: &str) -> Result<(), Error> {
        debug!("Deleting messages by sender: {}", sender);
        let filter = doc! { "sender": sender };
        match self.collection.delete_many(filter, None).await {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed to delete messages by sender: {}", sender);
                Err(e)
            }
        }
    }
}
