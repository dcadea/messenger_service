use futures::TryStreamExt;
use mongodb::bson::doc;
use mongodb::Database;

use super::{model::Message, Id};
use crate::{chat, message};

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
    pub async fn insert(&self, message: &Message) -> super::Result<Id> {
        let result = self.collection.insert_one(message).await?;
        let id = result
            .inserted_id
            .as_object_id()
            .ok_or(super::Error::IdNotPresent)?;
        Ok(Id(id.to_hex()))
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

    // pub async fn update(&self, id: &Id, text: &str) -> super::Result<()> {
    //     self.collection
    //         .update_one(doc! {"_id": id}, doc! {"$set": {"text": text}})
    //         .await?;
    //     Ok(())
    // }

    pub async fn delete(&self, id: &Id) -> super::Result<()> {
        self.collection.delete_one(doc! {"_id": id}).await?;
        Ok(())
    }

    // pub async fn mark_as_seen(&self, id: &Id) -> super::Result<()> {
    //     self.collection
    //         .update_one(doc! {"_id": id}, doc! {"$set": {"seen": true}})
    //         .await?;
    //     Ok(())
    // }
}
