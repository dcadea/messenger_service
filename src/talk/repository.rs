use futures::TryStreamExt;
use mongodb::bson::doc;

use super::model::Talk;
use crate::{message::model::LastMessage, talk, user};

const TALKS_COLLECTION: &str = "talks";

#[derive(Clone)]
pub struct TalkRepository {
    col: mongodb::Collection<Talk>,
}

impl TalkRepository {
    pub fn new(db: &mongodb::Database) -> Self {
        Self {
            col: db.collection(TALKS_COLLECTION),
        }
    }
}

impl TalkRepository {
    pub async fn find_by_id(&self, id: &talk::Id) -> super::Result<Talk> {
        let talk = self.col.find_one(doc! { "_id": id }).await?;

        talk.ok_or(talk::Error::NotFound(Some(id.to_owned())))
    }

    pub async fn find_by_sub(&self, sub: &user::Sub) -> super::Result<Vec<Talk>> {
        let cursor = self
            .col
            .find(doc! {"details.members": sub})
            .sort(doc! {"last_message.timestamp": -1})
            .await?;

        let talks: Vec<Talk> = cursor.try_collect().await?;

        Ok(talks)
    }

    pub async fn find_by_id_and_sub(&self, id: &talk::Id, sub: &user::Sub) -> super::Result<Talk> {
        let talk = self
            .col
            .find_one(doc! {
                "_id": id,
                "details.members": sub
            })
            .await?;

        talk.ok_or(talk::Error::NotFound(Some(id.to_owned())))
    }

    pub async fn create(&self, talk: Talk) -> super::Result<talk::Id> {
        let res = self.col.insert_one(talk).await?;

        if let Some(talk_id) = res.inserted_id.as_object_id() {
            return Ok(talk::Id(talk_id.to_hex()));
        }

        Err(talk::Error::NotCreated)
    }

    pub async fn delete(&self, id: &talk::Id) -> super::Result<()> {
        self.col.delete_one(doc! {"_id": id}).await?;
        Ok(())
    }
}

impl TalkRepository {
    pub async fn update_last_message(
        &self,
        id: &talk::Id,
        msg: Option<&LastMessage>,
    ) -> super::Result<()> {
        self.col
            .update_one(
                doc! { "_id": id },
                doc! {"$set": {
                    "last_message": msg,
                }},
            )
            .await?;
        Ok(())
    }

    pub async fn mark_as_seen(&self, id: &talk::Id) -> super::Result<()> {
        self.col
            .update_one(
                doc! { "_id": id },
                doc! {"$set": {
                    "last_message.seen": true,
                }},
            )
            .await?;
        Ok(())
    }
}
