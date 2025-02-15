use futures::TryStreamExt;
use log::error;
use mongodb::bson::doc;
use mongodb::Database;

use super::{model::Message, Id};
use crate::{chat, message};

const MESSAGES_COLLECTION: &str = "messages";

#[derive(Clone)]
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
    pub async fn insert(&self, message: &Message) -> super::Result<()> {
        let result = self.collection.insert_one(message).await?;
        result
            .inserted_id
            .as_object_id()
            .ok_or(super::Error::IdNotPresent)?;
        Ok(())
    }

    pub async fn insert_many(&self, messages: &[Message]) -> super::Result<()> {
        let result = self.collection.insert_many(messages).await?;

        if result.inserted_ids.len() != messages.len() {
            error!("not all messages persisted")
        }

        Ok(())
    }

    pub async fn find_by_id(&self, id: &Id) -> super::Result<Message> {
        self.collection
            .find_one(doc! {"_id": id})
            .await?
            .ok_or(message::Error::NotFound(Some(id.to_owned())))
    }

    pub async fn find_by_chat_id(&self, chat_id: &chat::Id) -> super::Result<Vec<Message>> {
        let cursor = self
            .collection
            .find(doc! {"chat_id": chat_id})
            .sort(doc! {"timestamp": -1})
            .await?;

        let messages = cursor.try_collect::<Vec<Message>>().await?;

        Ok(messages)
    }

    pub async fn find_by_chat_id_limited(
        &self,
        chat_id: &chat::Id,
        limit: usize,
    ) -> super::Result<Vec<Message>> {
        let cursor = self
            .collection
            .find(doc! {"chat_id": chat_id})
            .sort(doc! {"timestamp": -1})
            .limit(limit as i64)
            .await?;

        let messages = cursor.try_collect::<Vec<Message>>().await?;

        Ok(messages)
    }

    pub async fn find_by_chat_id_before(
        &self,
        chat_id: &chat::Id,
        before: i64,
    ) -> super::Result<Vec<Message>> {
        let cursor = self
            .collection
            .find(doc! {
                "chat_id": chat_id,
                "timestamp": {"$lt": before}
            })
            .sort(doc! {"timestamp": -1})
            .await?;

        let messages = cursor.try_collect::<Vec<Message>>().await?;

        Ok(messages)
    }

    pub async fn find_by_chat_id_limited_before(
        &self,
        chat_id: &chat::Id,
        limit: usize,
        before: i64,
    ) -> super::Result<Vec<Message>> {
        let cursor = self
            .collection
            .find(doc! {
                "chat_id": chat_id,
                "timestamp": {"$lt": before}
            })
            .sort(doc! {"timestamp": -1})
            .limit(limit as i64)
            .await?;

        let messages = cursor.try_collect::<Vec<Message>>().await?;

        Ok(messages)
    }

    pub async fn find_most_recent(&self, chat_id: &chat::Id) -> super::Result<Option<Message>> {
        let mut cursor = self
            .collection
            .find(doc! {"chat_id": chat_id})
            .sort(doc! {"timestamp": -1})
            .limit(1)
            .await?;

        let most_recent = cursor.try_next().await?;

        Ok(most_recent)
    }

    pub async fn update(&self, id: &Id, text: &str) -> super::Result<()> {
        self.collection
            .update_one(doc! {"_id": id}, doc! {"$set": {"text": text}})
            .await?;
        Ok(())
    }

    pub async fn delete(&self, id: &Id) -> super::Result<u64> {
        let deleted_count = self
            .collection
            .delete_one(doc! {"_id": id})
            .await?
            .deleted_count;

        Ok(deleted_count)
    }

    pub async fn delete_by_chat_id(&self, chat_id: &chat::Id) -> super::Result<()> {
        self.collection
            .delete_many(doc! {"chat_id": chat_id})
            .await?;

        Ok(())
    }

    pub async fn mark_as_seen(&self, ids: &[Id]) -> super::Result<()> {
        self.collection
            .update_many(
                doc! {"_id": {"$in": ids}, "seen": false},
                doc! {"$set": {"seen": true}},
            )
            .await?;
        Ok(())
    }
}
