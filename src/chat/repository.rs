use futures::TryStreamExt;
use mongodb::bson::doc;

use crate::message::model::LastMessage;
use crate::{chat, user};

use super::model::Chat;
use super::Id;

const CHATS_COLLECTION: &str = "chats";

#[derive(Clone)]
pub struct ChatRepository {
    collection: mongodb::Collection<Chat>,
}

impl ChatRepository {
    pub fn new(database: &mongodb::Database) -> Self {
        Self {
            collection: database.collection(CHATS_COLLECTION),
        }
    }
}

impl ChatRepository {
    pub async fn find_by_id(&self, id: &Id) -> super::Result<Chat> {
        let chat = self.collection.find_one(doc! { "_id": id }).await?;

        chat.ok_or(chat::Error::NotFound(Some(id.to_owned())))
    }

    /**
     * Find a chat where the user sub is a member
     * @param sub: The user sub
     */
    pub async fn find_by_sub(&self, sub: &user::Sub) -> super::Result<Vec<Chat>> {
        let cursor = self
            .collection
            .find(doc! {"members": sub})
            .sort(doc! {"last_message.timestamp": -1})
            .await?;

        let chats: Vec<Chat> = cursor.try_collect().await?;

        Ok(chats)
    }

    pub async fn find_by_id_and_sub(&self, id: &Id, sub: &user::Sub) -> super::Result<Chat> {
        let chat = self
            .collection
            .find_one(doc! {
                "_id": id,
                "members": sub
            })
            .await?;

        chat.ok_or(chat::Error::NotFound(Some(id.to_owned())))
    }

    pub async fn exists(&self, members: &[user::Sub; 2]) -> super::Result<bool> {
        let number_of_chats = self
            .collection
            .count_documents(doc! {
                "members": { "$all": members.to_vec() }
            })
            .await?;

        Ok(number_of_chats > 0)
    }

    pub async fn create(&self, chat: Chat) -> super::Result<Id> {
        let result = self.collection.insert_one(chat).await?;

        if let Some(chat_id) = result.inserted_id.as_object_id() {
            return Ok(Id(chat_id.to_hex()));
        }

        Err(chat::Error::NotCreated)
    }

    pub async fn delete(&self, id: &Id) -> super::Result<()> {
        self.collection.delete_one(doc! {"_id": id}).await?;
        Ok(())
    }
}

impl ChatRepository {
    pub async fn update_last_message(
        &self,
        id: &Id,
        msg: Option<&LastMessage>,
    ) -> super::Result<()> {
        self.collection
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
        self.collection
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
