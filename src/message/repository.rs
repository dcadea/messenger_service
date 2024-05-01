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

    pub(super) async fn insert(&self, message: &Message) -> Result<()> {
        self.collection.insert_one(message, None).await?;
        Ok(())
    }
}
