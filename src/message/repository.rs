use crate::error::ApiError;
use futures::TryStreamExt;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use mongodb::Database;
use std::sync::Arc;

use crate::message::model::Message;
use crate::result::Result;

pub struct MessageRepository {
    collection: mongodb::Collection<Message>,
}

impl MessageRepository {
    pub fn new(database: &Database) -> Arc<Self> {
        let collection = database.collection("messages");
        Self { collection }.into()
    }
}

impl MessageRepository {
    pub(super) async fn insert(&self, message: &Message) -> Result<()> {
        self.collection.insert_one(message, None).await?;
        Ok(())
    }

    pub(super) async fn find_all(&self) -> Result<Vec<Message>> {
        let cursor = self.collection.find(None, None).await?;
        cursor.try_collect().await.map_err(ApiError::from)
    }

    pub(super) async fn find_by_recipient(&self, recipient: &str) -> Result<Vec<Message>> {
        let cursor = self
            .collection
            .find(
                Some(doc! {
                    "recipient": recipient
                }),
                FindOptions::builder().sort(doc! {"timestamp": 1}).build(),
            )
            .await?;

        cursor.try_collect().await.map_err(ApiError::from)
    }
}
