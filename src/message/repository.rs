use futures::TryStreamExt;
use mongodb::bson::doc;
use mongodb::Database;

use super::model::{Message, MessageId};
use super::Result;
use crate::chat::model::ChatId;
use crate::message;

const MESSAGES_COLLECTION: &str = "messages";

pub struct MessageRepository {
    collection: mongodb::Collection<Message>,
}

impl MessageRepository {
    pub fn new(database: &Database) -> Self {
        Self {
            collection: database.collection(MESSAGES_COLLECTION),
        }
    }
}

impl MessageRepository {
    pub async fn insert(&self, message: &Message) -> Result<MessageId> {
        let result = self.collection.insert_one(message).await?;
        if let Some(id) = result.inserted_id.as_object_id() {
            return Ok(id.to_owned());
        }

        Err(message::Error::Unexpected(
            "Failed to insert message".to_owned(),
        ))
    }

    pub async fn find_by_id(&self, id: &MessageId) -> Result<Message> {
        self.collection
            .find_one(doc! {"_id": id})
            .await?
            .ok_or(message::Error::NotFound(Some(id.to_owned())))
    }

    pub async fn find_by_chat_id(&self, chat_id: &ChatId) -> Result<Vec<Message>> {
        let cursor = self
            .collection
            .find(doc! {"chat_id": chat_id})
            .sort(doc! {"timestamp": 1})
            .await?;

        let messages = cursor.try_collect::<Vec<Message>>().await?;

        Ok(messages)
    }

    pub async fn find_by_chat_id_limited(
        &self,
        chat_id: &ChatId,
        limit: usize,
    ) -> Result<Vec<Message>> {
        let cursor = self
            .collection
            .find(doc! {"chat_id": chat_id})
            .sort(doc! {"timestamp": -1})
            .limit(limit as i64)
            .await?;

        let messages = cursor
            .try_collect::<Vec<Message>>()
            .await
            .map(|mut messages| {
                messages.reverse();
                messages
            })?;

        Ok(messages)
    }

    pub async fn find_by_chat_id_before(
        &self,
        chat_id: &ChatId,
        before: i64,
    ) -> Result<Vec<Message>> {
        let cursor = self
            .collection
            .find(doc! {
                "chat_id": chat_id,
                "timestamp": {"$lt": before}
            })
            .sort(doc! {"timestamp": 1})
            .await?;

        let messages = cursor.try_collect::<Vec<Message>>().await?;

        Ok(messages)
    }

    pub async fn find_by_chat_id_limited_before(
        &self,
        chat_id: &ChatId,
        limit: usize,
        before: i64,
    ) -> Result<Vec<Message>> {
        let cursor = self
            .collection
            .find(doc! {
                "chat_id": chat_id,
                "timestamp": {"$lt": before}
            })
            .sort(doc! {"timestamp": -1})
            .limit(limit as i64)
            .await?;

        let messages = cursor
            .try_collect::<Vec<Message>>()
            .await
            .map(|mut messages| {
                messages.reverse();
                messages
            })?;

        Ok(messages)
    }

    pub async fn update(&self, id: &MessageId, text: &str) -> Result<()> {
        self.collection
            .update_one(doc! {"_id": id}, doc! {"$set": {"text": text}})
            .await?;
        Ok(())
    }

    pub async fn delete(&self, id: &MessageId) -> Result<()> {
        self.collection.delete_one(doc! {"_id": id}).await?;
        Ok(())
    }

    pub async fn mark_as_seen(&self, id: &MessageId) -> Result<()> {
        self.collection
            .update_one(doc! {"_id": id}, doc! {"$set": {"seen": true}})
            .await?;
        Ok(())
    }
}
