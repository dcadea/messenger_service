use futures::stream::TryStreamExt;
use log::{debug, error};
use mongodb::bson::doc;
use mongodb::error::Error;
use mongodb::results::InsertOneResult;
use mongodb::Database;

use crate::message::model::Message;

pub struct MessageRepository {
    collection: mongodb::Collection<Message>,
}

impl MessageRepository {
    pub fn new(database: Database) -> Self {
        let collection = database.collection("messages");
        Self { collection }
    }

    pub async fn insert(&self, message: &Message) -> Result<InsertOneResult, Error> {
        debug!("Inserting message: {:?}", message);
        match self.collection.insert_one(message, None).await {
            Ok(result) => Ok(result),
            Err(e) => {
                error!(
                    "Failed to insert message from: {} to: {} on {}",
                    message.sender(),
                    message.recipient(),
                    message.timestamp()
                );
                Err(e)
            }
        }
    }

    pub async fn find_by_recipient(&self, recipient: &str) -> Result<Vec<Message>, Error> {
        debug!("Finding messages by recipient: {}", recipient);
        let filter = doc! { "recipient": recipient };
        let cursor = self.collection.find(filter, None).await?;
        match cursor.try_collect().await {
            Ok(messages) => Ok(messages),
            Err(e) => {
                error!("Failed to find messages by recipient: {}", recipient);
                Err(e)
            }
        }
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
