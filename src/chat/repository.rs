use futures::stream::TryStreamExt;
use mongodb::bson::doc;

use crate::user::model::UserSub;

use super::error::ChatError;
use super::model::{Chat, ChatId, Members};
use super::Result;

pub struct ChatRepository {
    collection: mongodb::Collection<Chat>,
}

impl ChatRepository {
    pub fn new(database: &mongodb::Database) -> Self {
        Self {
            collection: database.collection("chats"),
        }
    }
}

impl ChatRepository {
    pub async fn insert(&self, chat: &Chat) -> Result<ChatId> {
        let result = self.collection.insert_one(chat, None).await?;
        if let Some(id) = result.inserted_id.as_object_id() {
            return Ok(id.to_owned());
        }

        Err(ChatError::Unexpected("Failed to insert chat".to_owned()))
    }

    pub async fn update_last_message(&self, id: &ChatId, text: &str) -> Result<()> {
        let filter = doc! { "_id": id };
        let update = doc! {"$set": { "last_message": text }};
        self.collection.update_one(filter, update, None).await?;
        Ok(())
    }

    pub async fn find_by_id(&self, id: ChatId) -> Result<Chat> {
        let filter = doc! { "_id": id };
        let result = self.collection.find_one(Some(filter), None).await?;
        result.ok_or(ChatError::NotFound(Some(id)))
    }

    pub async fn find_by_sub(&self, sub: &UserSub) -> Result<Vec<Chat>> {
        let filter = doc! {
            "$or": [
                { "members.me": sub },
                { "members.you": sub },
            ]
        };
        let cursor = self.collection.find(Some(filter), None).await?;
        cursor.try_collect().await.map_err(ChatError::from)
    }

    pub async fn find_id_by_members(&self, members: &Members) -> Result<ChatId> {
        let me = &members.me;
        let you = &members.you;

        let filter = doc! {
            "$or": [
                { "members.me": me, "members.you": you },
                { "members.me": you, "members.you": me },
            ]
        };

        let result = self.collection.find_one(Some(filter), None).await?;
        if let Some(chat) = result {
            if let Some(id) = chat.id {
                return Ok(id);
            }
        }

        Err(ChatError::NotFound(None))
    }
}
