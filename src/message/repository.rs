use futures::TryStreamExt;
use log::error;
use mongodb::Database;
use mongodb::bson::doc;

use super::{Id, model::Message};
use crate::{chat, message};

const MESSAGES_COLLECTION: &str = "messages";

#[derive(Clone)]
pub struct MessageRepository {
    col: mongodb::Collection<Message>,
}

impl MessageRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            col: db.collection(MESSAGES_COLLECTION),
        }
    }
}

impl MessageRepository {
    pub async fn insert(&self, msg: &Message) -> super::Result<()> {
        let result = self.col.insert_one(msg).await?;
        result
            .inserted_id
            .as_object_id()
            .ok_or(super::Error::IdNotPresent)?;
        Ok(())
    }

    pub async fn insert_many(&self, msgs: &[Message]) -> super::Result<()> {
        let result = self.col.insert_many(msgs).await?;

        if result.inserted_ids.len() != msgs.len() {
            error!("not all messages persisted")
        }

        Ok(())
    }

    pub async fn find_by_id(&self, id: &Id) -> super::Result<Message> {
        self.col
            .find_one(doc! {"_id": id})
            .await?
            .ok_or(message::Error::NotFound(Some(id.to_owned())))
    }

    pub async fn find_by_chat_id(&self, chat_id: &chat::Id) -> super::Result<Vec<Message>> {
        let cursor = self
            .col
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
            .col
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
            .col
            .find(doc! {
                "chat_id": chat_id,
                "timestamp": {"$lt": before}
            })
            .sort(doc! {"timestamp": -1})
            .await?;

        let msgs = cursor.try_collect::<Vec<Message>>().await?;

        Ok(msgs)
    }

    pub async fn find_by_chat_id_limited_before(
        &self,
        chat_id: &chat::Id,
        limit: usize,
        before: i64,
    ) -> super::Result<Vec<Message>> {
        let cursor = self
            .col
            .find(doc! {
                "chat_id": chat_id,
                "timestamp": {"$lt": before}
            })
            .sort(doc! {"timestamp": -1})
            .limit(limit as i64)
            .await?;

        let msgs = cursor.try_collect::<Vec<Message>>().await?;

        Ok(msgs)
    }

    pub async fn find_most_recent(&self, chat_id: &chat::Id) -> super::Result<Option<Message>> {
        let mut cursor = self
            .col
            .find(doc! {"chat_id": chat_id})
            .sort(doc! {"timestamp": -1})
            .limit(1)
            .await?;

        let msg = cursor.try_next().await?;

        Ok(msg)
    }

    pub async fn update(&self, id: &Id, text: &str) -> super::Result<()> {
        self.col
            .update_one(doc! {"_id": id}, doc! {"$set": {"text": text}})
            .await?;
        Ok(())
    }

    pub async fn delete(&self, id: &Id) -> super::Result<u64> {
        let count = self.col.delete_one(doc! {"_id": id}).await?.deleted_count;

        Ok(count)
    }

    pub async fn delete_by_chat_id(&self, chat_id: &chat::Id) -> super::Result<()> {
        self.col.delete_many(doc! {"chat_id": chat_id}).await?;

        Ok(())
    }

    pub async fn mark_as_seen(&self, ids: &[Id]) -> super::Result<()> {
        self.col
            .update_many(
                doc! {"_id": {"$in": ids}, "seen": false},
                doc! {"$set": {"seen": true}},
            )
            .await?;
        Ok(())
    }
}
