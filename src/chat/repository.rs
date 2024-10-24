use futures::stream::TryStreamExt;
use mongodb::bson::doc;

use crate::{chat, user};

use super::model::Chat;
use super::Id;

const CHATS_COLLECTION: &str = "chats";

pub struct ChatRepository {
    collection: mongodb::Collection<Chat>,
}

impl ChatRepository {
    pub fn new(database: &mongodb::Database) -> Self {
        Self {
            collection: database.collection(CHATS_COLLECTION),
        }
    }
}

impl ChatRepository {
    pub async fn update_last_message(&self, id: &Id, text: &str) -> super::Result<()> {
        self.collection
            .update_one(
                doc! { "_id": id },
                doc! {"$set": {
                    "last_message": text,
                    "updated_at": chrono::Utc::now().timestamp(),
                }},
            )
            .await?;
        Ok(())
    }

    pub async fn find_by_id(&self, id: &Id) -> super::Result<Chat> {
        self.collection
            .find_one(doc! { "_id": id })
            .await?
            .ok_or(chat::Error::NotFound(Some(id.to_owned())))
    }

    /**
     * Find a chat where the user sub is a member
     * @param sub: The user sub
     */
    pub async fn find_by_sub(&self, sub: &user::Sub) -> super::Result<Vec<Chat>> {
        let cursor = self
            .collection
            .find(doc! {"members": sub})
            .sort(doc! {"updated_at": -1})
            .await?;

        let chats = cursor.try_collect::<Vec<Chat>>().await?;

        Ok(chats)
    }

    pub async fn find_by_id_and_sub(&self, id: &Id, sub: &user::Sub) -> super::Result<Chat> {
        self.collection
            .find_one(doc! {
                "_id": id,
                "members": sub
            })
            .await?
            .ok_or(chat::Error::NotFound(Some(id.to_owned())))
    }

    pub async fn find_id_by_members(&self, members: [user::Sub; 2]) -> super::Result<Id> {
        let result = self
            .collection
            .find_one(doc! {
                "members": { "$all": members.to_vec() }
            })
            .await?;

        if let Some(chat) = result {
            if let Some(id) = chat.id {
                return Ok(id);
            }
        }

        Err(chat::Error::NotFound(None))
    }
}
