use futures::stream::TryStreamExt;
use std::sync::Arc;

use mongodb::bson::doc;

use crate::chat::model::Chat;
use crate::result::Result;

pub struct ChatRepository {
    collection: mongodb::Collection<Chat>,
}

impl ChatRepository {
    pub fn new(database: &mongodb::Database) -> Arc<Self> {
        let collection = database.collection("chats");
        Self { collection }.into()
    }

    pub async fn find_by_username(&self, username: &str) -> Result<Vec<Chat>> {
        let cursor = self
            .collection
            .find(
                Some(doc! {
                    "username": username
                }),
                None,
            )
            .await?;

        let chats: Vec<Chat> = cursor.try_collect().await?;

        Ok(chats)
    }
}
