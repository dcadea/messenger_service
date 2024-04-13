use futures::stream::TryStreamExt;
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use mongodb::error::Error;
use mongodb::options::FindOptions;
use mongodb::Database;
use std::sync::Arc;

use crate::message::model::Message;

pub struct MessageRepository {
    collection: mongodb::Collection<Message>,
}

impl MessageRepository {
    pub fn new(database: &Database) -> Arc<Self> {
        let collection = database.collection("messages");
        Self { collection }.into()
    }

    pub async fn insert(&self, message: &Message) -> Result<Option<ObjectId>, Error> {
        self.collection
            .insert_one(message, None)
            .await
            .map(|r| r.inserted_id.as_object_id())
    }

    pub async fn find_by_recipient(&self, recipient: &str) -> Result<Vec<Message>, Error> {
        let filter = doc! { "recipient": recipient };
        let asc_by_timestamp = FindOptions::builder().sort(doc! { "timestamp": 1 }).build();
        let cursor = self.collection.find(filter, asc_by_timestamp).await?;

        cursor.try_collect().await
    }

    pub async fn delete_by_sender(&self, sender: &str) -> Result<bool, Error> {
        let filter = doc! { "sender": sender };

        self.collection
            .delete_many(filter, None)
            .await
            .map(|r| r.deleted_count > 0)
    }
}
