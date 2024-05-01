use futures::stream::TryStreamExt;
use std::sync::Arc;

use mongodb::bson::doc;

use crate::chat::model::Chat;
use crate::error::ApiError;
use crate::result::Result;

pub struct ChatRepository {
    collection: mongodb::Collection<Chat>,
}

impl ChatRepository {
    pub fn new(database: &mongodb::Database) -> Arc<Self> {
        let collection = database.collection("chats");
        Self { collection }.into()
    }
}

impl ChatRepository {
    pub(super) async fn insert(&self, chat: &Chat) -> Result<()> {
        self.collection.insert_one(chat, None).await?;
        Ok(())
    }

    pub(super) async fn find_all(&self) -> Result<Vec<Chat>> {
        let cursor = self.collection.find(None, None).await?;
        cursor.try_collect().await.map_err(ApiError::from)
    }

    pub(super) async fn find_by_username(&self, username: &str) -> Result<Vec<Chat>> {
        let cursor = self
            .collection
            .find(
                Some(doc! {
                    "username": username
                }),
                None,
            )
            .await?;

        cursor.try_collect().await.map_err(ApiError::from)
    }
}
