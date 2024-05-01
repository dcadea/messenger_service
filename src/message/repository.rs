use mongodb::bson::oid::ObjectId;
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

    pub async fn insert(&self, message: &Message) -> Result<Option<ObjectId>> {
        let inserted_id = self
            .collection
            .insert_one(message, None)
            .await?
            .inserted_id
            .as_object_id();

        Ok(inserted_id)
    }
}
