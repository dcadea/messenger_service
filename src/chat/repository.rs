use futures::stream::TryStreamExt;

use mongodb::bson::{doc, Document};

use crate::chat::model::Chat;
use crate::error::ApiError;
use crate::result::Result;

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
    pub async fn insert(&self, chat: &Chat) -> Result<()> {
        self.collection.insert_one(chat, None).await?;
        Ok(())
    }

    pub async fn find_by_sender(&self, sender: &str) -> Result<Vec<Chat>> {
        self.find(doc! { "sender": sender }).await
    }

    async fn find(&self, filter: Document) -> Result<Vec<Chat>> {
        let cursor = self.collection.find(Some(filter), None).await?;
        cursor.try_collect().await.map_err(ApiError::from)
    }
}
