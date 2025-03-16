use futures::TryStreamExt;
use mongodb::bson::doc;

use crate::message::model::LastMessage;
use crate::{chat, user};

use super::Id;
use super::model::Chat;

const CHATS_COLLECTION: &str = "chats";

#[derive(Clone)]
pub struct ChatRepository {
    col: mongodb::Collection<Chat>,
}

impl ChatRepository {
    pub fn new(db: &mongodb::Database) -> Self {
        Self {
            col: db.collection(CHATS_COLLECTION),
        }
    }
}

impl ChatRepository {
    pub async fn exists(&self, members: &[user::Sub; 2]) -> super::Result<bool> {
        let count = self
            .col
            .count_documents(doc! {
                "members": { "$all": members.to_vec() }
            })
            .await?;

        Ok(count > 0)
    }
}
