use anyhow::Context;
use futures::TryStreamExt;
use mongodb::bson::doc;

use crate::{chat, user};

use super::model::Chat;
use super::Id;

const CHATS_COLLECTION: &str = "chats";

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
    pub async fn update_last_message(&self, id: &Id, text: &str) -> super::Result<()> {
        self.collection
            .update_one(
                doc! { "_id": id },
                doc! {"$set": {
                    "last_message": text,
                    "updated_at": chrono::Utc::now().timestamp(),
                }},
            )
            .await
            .with_context(|| format!("Failed to update last message for chat: {id:?}"))?;
        Ok(())
    }

    pub async fn find_by_id(&self, id: &Id) -> super::Result<Chat> {
        let chat = self
            .collection
            .find_one(doc! { "_id": id })
            .await
            .with_context(|| format!("Failed to find chat by id: {id:?}"))?;

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
            .sort(doc! {"updated_at": -1})
            .await
            .with_context(|| format!("Failed to find chats by sub: {sub:?}"))?;

        let chats: Vec<Chat> = cursor
            .try_collect()
            .await
            .with_context(|| format!("Failed to collect chats for sub: {sub:?}"))?;

        Ok(chats)
    }

    pub async fn find_by_id_and_sub(&self, id: &Id, sub: &user::Sub) -> super::Result<Chat> {
        let chat = self
            .collection
            .find_one(doc! {
                "_id": id,
                "members": sub
            })
            .await
            .with_context(|| format!("Failed to find chat by id: {id:?} and sub: {sub:?}"))?;

        chat.ok_or(chat::Error::NotFound(Some(id.to_owned())))
    }

    pub async fn find_id_by_members(&self, members: [user::Sub; 2]) -> super::Result<Id> {
        let chat = self
            .collection
            .find_one(doc! {
                "members": { "$all": members.to_vec() }
            })
            .await
            .with_context(|| format!("Failed to find chat by members: {members:?}"))?;

        if let Some(chat) = chat {
            if let Some(id) = chat.id {
                return Ok(id);
            }
        }

        Err(chat::Error::NotFound(None))
    }

    pub async fn exists(&self, members: &[user::Sub; 2]) -> super::Result<bool> {
        let number_of_chats = self
            .collection
            .count_documents(doc! {
                "members": { "$all": members.to_vec() }
            })
            .await
            .with_context(|| format!("Failed to find chat by members: {members:?}"))?;

        Ok(number_of_chats > 0)
    }

    pub async fn create(&self, chat: Chat) -> super::Result<Id> {
        let result = self
            .collection
            .insert_one(chat)
            .await
            .with_context(|| "Failed to create chat")?;

        if let Some(chat_id) = result.inserted_id.as_object_id() {
            return Ok(Id(chat_id.to_hex()));
        }

        Err(chat::Error::NotCreated)
    }
}
