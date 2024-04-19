use mongodb::bson::oid::ObjectId;
use mongodb::error::Error;
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
}
