use futures::TryStreamExt;
use mongodb::bson::{doc, Document};
use mongodb::options::FindOptions;
use mongodb::Database;

use crate::error::ApiError;
use crate::message::model::Message;
use crate::result::Result;

pub struct MessageRepository {
    collection: mongodb::Collection<Message>,
}

impl MessageRepository {
    pub fn new(database: &Database) -> Self {
        Self {
            collection: database.collection("messages"),
        }
    }
}

impl MessageRepository {
    pub(super) async fn insert(&self, message: &Message) -> Result<()> {
        self.collection.insert_one(message, None).await?;
        Ok(())
    }

    pub(super) async fn find_by_participants(
        &self,
        participants: &Vec<String>,
    ) -> Result<Vec<Message>> {
        let document = doc! { // FIXME
            "sender": {"$in": participants},
            "recipient": {"$in": participants}
        };
        self.find(document).await
    }

    async fn find(&self, filter: Document) -> Result<Vec<Message>> {
        let cursor = self
            .collection
            .find(
                Some(filter),
                FindOptions::builder().sort(doc! {"timestamp": 1}).build(),
            )
            .await?;

        cursor.try_collect().await.map_err(ApiError::from)
    }
}
