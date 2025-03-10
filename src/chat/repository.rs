use futures::TryStreamExt;
use mongodb::bson::doc;

use crate::message::model::LastMessage;
use crate::{chat, user};

use super::Id;
use super::model::Chat;

const CHATS_COLLECTION: &str = "chats";

#[derive(Clone)]
pub struct ChatRepository {
    col: mongodb::Collection<Chat>,
}

impl ChatRepository {
    pub fn new(db: &mongodb::Database) -> Self {
        Self {
            col: db.collection(CHATS_COLLECTION),
        }
    }
}

impl ChatRepository {
    pub async fn find_by_id(&self, id: &Id) -> super::Result<Chat> {
        let chat = self.col.find_one(doc! { "_id": id }).await?;

        chat.ok_or(chat::Error::NotFound(Some(id.to_owned())))
    }

    /**
     * Find chats where the user sub is a member
     * @param sub: The user sub
     */
    pub async fn find_by_sub(&self, sub: &user::Sub) -> super::Result<Vec<Chat>> {
        let cursor = self
            .col
            .find(doc! {"members": sub})
            .sort(doc! {"last_message.timestamp": -1})
            .await?;

        let chats: Vec<Chat> = cursor.try_collect().await?;

        Ok(chats)
    }

    pub async fn find_by_id_and_sub(&self, id: &Id, sub: &user::Sub) -> super::Result<Chat> {
        let chat = self
            .col
            .find_one(doc! {
                "_id": id,
                "members": sub
            })
            .await?;

        chat.ok_or(chat::Error::NotFound(Some(id.to_owned())))
    }

    pub async fn exists(&self, members: &[user::Sub; 2]) -> super::Result<bool> {
        let count = self
            .col
            .count_documents(doc! {
                "members": { "$all": members.to_vec() }
            })
            .await?;

        Ok(count > 0)
    }

    pub async fn create(&self, chat: Chat) -> super::Result<Id> {
        let res = self.col.insert_one(chat).await?;

        if let Some(chat_id) = res.inserted_id.as_object_id() {
            return Ok(Id(chat_id.to_hex()));
        }

        Err(chat::Error::NotCreated)
    }

    pub async fn delete(&self, id: &Id) -> super::Result<()> {
        self.col.delete_one(doc! {"_id": id}).await?;
        Ok(())
    }
}

impl ChatRepository {
    pub async fn update_last_message(
        &self,
        id: &Id,
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

    pub async fn mark_as_seen(&self, id: &Id) -> super::Result<()> {
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
