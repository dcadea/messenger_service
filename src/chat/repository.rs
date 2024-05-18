use futures::stream::TryStreamExt;

use mongodb::bson::{doc, Document};

use crate::error::ApiError;
use crate::result::Result;

use super::model::Chat;
use super::model::ChatId;

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

        Err(ApiError::InternalServerError(
            "Failed to insert chat".to_owned(),
        ))
    }

    pub async fn find_by_sender(&self, sender: &str) -> Result<Vec<Chat>> {
        self.find(doc! { "sender": sender }).await
    }

    async fn find(&self, filter: Document) -> Result<Vec<Chat>> {
        let cursor = self.collection.find(Some(filter), None).await?;
        cursor.try_collect().await.map_err(ApiError::from)
    }
}
