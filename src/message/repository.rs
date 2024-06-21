use futures::TryStreamExt;
use mongodb::bson::{doc, Document};
use mongodb::options::FindOptions;
use mongodb::Database;

use crate::chat::model::ChatId;

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

        let find_options = FindOptions::builder().sort(doc! {"timestamp": 1}).build();

        self.find_messages_filtered(filter, find_options).await
    }

    pub async fn find_by_chat_id_limited(
        &self,
        chat_id: &ChatId,
        limit: usize,
    ) -> Result<Vec<Message>> {
        let filter = doc! {"chat_id": chat_id};

        let find_options = FindOptions::builder()
            .sort(doc! {"timestamp": -1})
            .limit(limit as i64)
            .build();

        self.find_messages_filtered(filter, find_options).await
            .map(|mut messages| {
                messages.reverse();
                messages
            })
    }

    pub async fn find_by_chat_id_before(
        &self,
        chat_id: &ChatId,
        before: i64,
    ) -> Result<Vec<Message>> {
        let filter = doc! {
            "chat_id": chat_id,
            "timestamp": {"$lt": before}
        };

        let find_options = FindOptions::builder().sort(doc! {"timestamp": 1}).build();

        self.find_messages_filtered(filter, find_options).await
    }

    pub async fn find_by_chat_id_limited_before(
        &self,
        chat_id: &ChatId,
        limit: usize,
        before: i64,
    ) -> Result<Vec<Message>> {
        let filter = doc! {
            "chat_id": chat_id,
            "timestamp": {"$lt": before}
        };

        let find_options = FindOptions::builder()
            .sort(doc! {"timestamp": -1})
            .limit(limit as i64)
            .build();

        self.find_messages_filtered(filter, find_options).await
            .map(|mut messages| {
                messages.reverse();
                messages
            })
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

impl MessageRepository {
    async fn find_messages_filtered(
        &self,
        filter: Document,
        find_options: FindOptions,
    ) -> Result<Vec<Message>> {
        let cursor = self.collection.find(Some(filter), find_options).await?;
        cursor.try_collect().await.map_err(MessageError::from)
    }
}
