use futures::TryStreamExt;
use log::error;
use mongodb::Database;
use mongodb::bson::doc;

use super::{Id, model::Message};
use crate::{message, talk};

const MESSAGES_COLLECTION: &str = "messages";

#[async_trait::async_trait]
pub trait MessageRepository {
    async fn insert(&self, msg: &Message) -> super::Result<()>;

    async fn insert_many(&self, msgs: &[Message]) -> super::Result<()>;

    async fn find_by_id(&self, id: &Id) -> super::Result<Message>;

    async fn find_by_talk_id(&self, talk_id: &talk::Id) -> super::Result<Vec<Message>>;

    async fn find_by_talk_id_limited(
        &self,
        talk_id: &talk::Id,
        limit: usize,
    ) -> super::Result<Vec<Message>>;

    async fn find_by_talk_id_before(
        &self,
        talk_id: &talk::Id,
        before: i64,
    ) -> super::Result<Vec<Message>>;

    async fn find_by_talk_id_limited_before(
        &self,
        talk_id: &talk::Id,
        limit: usize,
        before: i64,
    ) -> super::Result<Vec<Message>>;

    async fn find_most_recent(&self, talk_id: &talk::Id) -> super::Result<Option<Message>>;

    async fn update(&self, id: &Id, text: &str) -> super::Result<()>;

    async fn delete(&self, id: &Id) -> super::Result<u64>;

    async fn delete_by_talk_id(&self, talk_id: &talk::Id) -> super::Result<()>;

    async fn mark_as_seen(&self, ids: &[Id]) -> super::Result<()>;
}

#[derive(Clone)]
pub struct MongoMessageRepository {
    col: mongodb::Collection<Message>,
}

impl MongoMessageRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            col: db.collection(MESSAGES_COLLECTION),
        }
    }
}

#[async_trait::async_trait]
impl MessageRepository for MongoMessageRepository {
    async fn insert(&self, msg: &Message) -> super::Result<()> {
        let result = self.col.insert_one(msg).await?;
        result
            .inserted_id
            .as_object_id()
            .ok_or(super::Error::IdNotPresent)?;
        Ok(())
    }

    async fn insert_many(&self, msgs: &[Message]) -> super::Result<()> {
        let result = self.col.insert_many(msgs).await?;

        if result.inserted_ids.len() != msgs.len() {
            error!("not all messages persisted");
        }

        Ok(())
    }

    async fn find_by_id(&self, id: &Id) -> super::Result<Message> {
        self.col
            .find_one(doc! {"_id": id})
            .await?
            .ok_or(message::Error::NotFound(Some(id.to_owned())))
    }

    async fn find_by_talk_id(&self, talk_id: &talk::Id) -> super::Result<Vec<Message>> {
        let cursor = self
            .col
            .find(doc! {"talk_id": talk_id})
            .sort(doc! {"timestamp": -1})
            .await?;

        let messages = cursor.try_collect::<Vec<Message>>().await?;

        Ok(messages)
    }

    async fn find_by_talk_id_limited(
        &self,
        talk_id: &talk::Id,
        limit: usize,
    ) -> super::Result<Vec<Message>> {
        let cursor = self
            .col
            .find(doc! {"talk_id": talk_id})
            .sort(doc! {"timestamp": -1})
            .limit(limit as i64)
            .await?;

        let messages = cursor.try_collect::<Vec<Message>>().await?;

        Ok(messages)
    }

    async fn find_by_talk_id_before(
        &self,
        talk_id: &talk::Id,
        before: i64,
    ) -> super::Result<Vec<Message>> {
        let cursor = self
            .col
            .find(doc! {
                "talk_id": talk_id,
                "timestamp": {"$lt": before}
            })
            .sort(doc! {"timestamp": -1})
            .await?;

        let msgs = cursor.try_collect::<Vec<Message>>().await?;

        Ok(msgs)
    }

    async fn find_by_talk_id_limited_before(
        &self,
        talk_id: &talk::Id,
        limit: usize,
        before: i64,
    ) -> super::Result<Vec<Message>> {
        let cursor = self
            .col
            .find(doc! {
                "talk_id": talk_id,
                "timestamp": {"$lt": before}
            })
            .sort(doc! {"timestamp": -1})
            .limit(limit as i64)
            .await?;

        let msgs = cursor.try_collect::<Vec<Message>>().await?;

        Ok(msgs)
    }

    async fn find_most_recent(&self, talk_id: &talk::Id) -> super::Result<Option<Message>> {
        let mut cursor = self
            .col
            .find(doc! {"talk_id": talk_id})
            .sort(doc! {"timestamp": -1})
            .limit(1)
            .await?;

        let msg = cursor.try_next().await?;

        Ok(msg)
    }

    async fn update(&self, id: &Id, text: &str) -> super::Result<()> {
        self.col
            .update_one(doc! {"_id": id}, doc! {"$set": {"text": text}})
            .await?;
        Ok(())
    }

    async fn delete(&self, id: &Id) -> super::Result<u64> {
        let count = self.col.delete_one(doc! {"_id": id}).await?.deleted_count;

        Ok(count)
    }

    async fn delete_by_talk_id(&self, talk_id: &talk::Id) -> super::Result<()> {
        self.col.delete_many(doc! {"talk_id": talk_id}).await?;

        Ok(())
    }

    async fn mark_as_seen(&self, ids: &[Id]) -> super::Result<()> {
        self.col
            .update_many(
                doc! {"_id": {"$in": ids}, "seen": false},
                doc! {"$set": {"seen": true}},
            )
            .await?;
        Ok(())
    }
}
