use futures::TryStreamExt;
use mongodb::bson::doc;

use super::model::Talk;
use crate::{message::model::LastMessage, talk, user};

const TALKS_COLLECTION: &str = "talks";

#[async_trait::async_trait]
pub trait TalkRepository {
    async fn find_by_id(&self, id: &talk::Id) -> super::Result<Talk>;

    async fn find_by_sub(&self, sub: &user::Sub) -> super::Result<Vec<Talk>>;

    async fn find_by_sub_and_kind(
        &self,
        sub: &user::Sub,
        kind: &talk::Kind,
    ) -> super::Result<Vec<Talk>>;

    async fn find_by_id_and_sub(&self, id: &talk::Id, sub: &user::Sub) -> super::Result<Talk>;

    async fn create(&self, talk: Talk) -> super::Result<()>;

    async fn delete(&self, id: &talk::Id) -> super::Result<()>;

    async fn exists(&self, members: &[user::Sub; 2]) -> super::Result<bool>;

    async fn update_last_message(
        &self,
        id: &talk::Id,
        msg: Option<&LastMessage>,
    ) -> super::Result<()>;

    async fn mark_as_seen(&self, id: &talk::Id) -> super::Result<()>;
}

#[derive(Clone)]
pub struct MongoTalkRepository {
    col: mongodb::Collection<Talk>,
}

impl MongoTalkRepository {
    pub fn new(db: &mongodb::Database) -> Self {
        Self {
            col: db.collection(TALKS_COLLECTION),
        }
    }
}

#[async_trait::async_trait]
impl TalkRepository for MongoTalkRepository {
    async fn find_by_id(&self, id: &talk::Id) -> super::Result<Talk> {
        let talk = self.col.find_one(doc! { "_id": id }).await?;

        talk.ok_or(talk::Error::NotFound(Some(id.to_owned())))
    }

    async fn find_by_sub(&self, sub: &user::Sub) -> super::Result<Vec<Talk>> {
        let cursor = self
            .col
            .find(doc! {"details.members": sub})
            .sort(doc! {"last_message.timestamp": -1})
            .await?;

        let talks: Vec<Talk> = cursor.try_collect().await?;

        Ok(talks)
    }

    async fn find_by_sub_and_kind(
        &self,
        sub: &user::Sub,
        kind: &talk::Kind,
    ) -> super::Result<Vec<Talk>> {
        let cursor = self
            .col
            .find(doc! {
                "kind": kind.as_str(),
                "details.members": sub,
            })
            .sort(doc! {"last_message.timestamp": -1})
            .await?;

        let talks: Vec<Talk> = cursor.try_collect().await?;

        Ok(talks)
    }

    async fn find_by_id_and_sub(&self, id: &talk::Id, sub: &user::Sub) -> super::Result<Talk> {
        let talk = self
            .col
            .find_one(doc! {
                "_id": id,
                "details.members": sub
            })
            .await?;

        talk.ok_or(talk::Error::NotFound(Some(id.to_owned())))
    }

    async fn create(&self, talk: Talk) -> super::Result<()> {
        let res = self.col.insert_one(talk).await?;

        if let mongodb::bson::Bson::Null = res.inserted_id {
            return Err(talk::Error::NotCreated);
        }

        Ok(())
    }

    async fn delete(&self, id: &talk::Id) -> super::Result<()> {
        self.col.delete_one(doc! {"_id": id}).await?;
        Ok(())
    }

    async fn exists(&self, members: &[user::Sub; 2]) -> super::Result<bool> {
        let count = self
            .col
            .count_documents(doc! { "details.members": { "$all": members.to_vec() } })
            .await?;

        Ok(count > 0)
    }

    async fn update_last_message(
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

    async fn mark_as_seen(&self, id: &talk::Id) -> super::Result<()> {
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
