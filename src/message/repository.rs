use futures::TryStreamExt;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use mongodb::Database;

use super::model::{Message, MessageId};
use crate::error::ApiError;
use crate::result::Result;

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

        Err(ApiError::InternalServerError(
            "Failed to insert message".to_owned(),
        ))
    }

    pub async fn find_by_id(&self, id: &MessageId) -> Result<Message> {
        let filter = doc! {"_id": id};
        let message = self.collection.find_one(Some(filter), None).await?;

        match message {
            Some(message) => Ok(message),
            None => Err(ApiError::NotFound("Message not found".to_owned())),
        }
    }

    pub async fn find_by_participants(&self, participants: &Vec<String>) -> Result<Vec<Message>> {
        let filter = doc! {
            "sender": {"$in": participants},
            "recipient": {"$in": participants}
        };

        let cursor = self
            .collection
            .find(
                Some(filter),
                FindOptions::builder().sort(doc! {"timestamp": 1}).build(),
            )
            .await?;

        cursor.try_collect().await.map_err(ApiError::from)
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
