use crate::chat::model::ChatId;
use futures::TryStreamExt;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use mongodb::Database;

use super::error::MessageError;
use super::model::{Message, MessageId};
use super::Result;

pub struct MessageRepository {
    collection: mongodb::Collection<Message>,
}

impl MessageRepository {
    pub fn new(database: &Database) -> Self {
        Self {
            collection: database.collection("messages"),
        }
    }
}

impl MessageRepository {
    pub async fn insert(&self, message: &Message) -> Result<MessageId> {
        let result = self.collection.insert_one(message, None).await?;
        if let Some(id) = result.inserted_id.as_object_id() {
            return Ok(id.to_owned());
        }

        Err(MessageError::Unexpected(
            "Failed to insert message".to_owned(),
        ))
    }

    pub async fn find_by_id(&self, id: MessageId) -> Result<Message> {
        let filter = doc! {"_id": id};
        let result = self.collection.find_one(Some(filter), None).await?;
        result.ok_or(MessageError::NotFound(Some(id)))
    }

    pub async fn find_by_chat_id(&self, chat_id: &ChatId) -> Result<Vec<Message>> {
        let filter = doc! {"chat_id": chat_id};
        let cursor = self
            .collection
            .find(
                Some(filter),
                FindOptions::builder().sort(doc! {"timestamp": 1}).build(),
            )
            .await?;

        cursor.try_collect().await.map_err(MessageError::from)
    }

    pub async fn update(&self, id: &MessageId, text: &str) -> Result<()> {
        let filter = doc! {"_id": id};
        let update = doc! {"$set": {"text": text}};
        self.collection.update_one(filter, update, None).await?;

        Ok(())
    }

    pub async fn delete(&self, id: &MessageId) -> Result<()> {
        let filter = doc! {"_id": id};
        self.collection.delete_one(filter, None).await?;

        Ok(())
    }

    pub async fn mark_as_seen(&self, id: &MessageId) -> Result<()> {
        let filter = doc! {"_id": id};
        let update = doc! {"$set": {"seen": true}};
        self.collection.update_one(filter, update, None).await?;

        Ok(())
    }
}
