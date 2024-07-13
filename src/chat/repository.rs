use futures::stream::TryStreamExt;
use mongodb::bson::doc;

use crate::user::model::UserSub;

use super::error::ChatError;
use super::model::{Chat, ChatId};
use super::Result;

pub struct ChatRepository {
    collection: mongodb::Collection<Chat>,
}

impl ChatRepository {
    pub fn new(database: &mongodb::Database) -> Self {
        Self {
            collection: database.collection("chats"),
        }
    }
}

impl ChatRepository {
    /**
     * Insert a new chat into the database
     * @param chat: The chat to insert
     */
    pub async fn insert(&self, chat: &Chat) -> Result<Chat> {
        let result = self.collection.insert_one(chat).await?;
        if let Some(id) = result.inserted_id.as_object_id() {
            return self.find_by_id(&id).await;
        }

        Err(ChatError::Unexpected("Failed to insert chat".to_owned()))
    }

    /**
     * Update the last message of a chat
     * @param id: The id of the chat
     * @param text: The text of the last message
     * @param updated_at: The timestamp of the last message
     */
    pub async fn update_last_message(&self, id: &ChatId, text: &str) -> Result<()> {
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

    pub async fn find_by_id(&self, id: &ChatId) -> Result<Chat> {
        self.collection
            .find_one(doc! { "_id": id })
            .await?
            .ok_or(ChatError::NotFound(Some(*id)))
    }

    /**
     * Find a chat where the user sub is a member
     * @param sub: The user sub
     */
    pub async fn find_by_sub(&self, sub: &UserSub) -> Result<Vec<Chat>> {
        let cursor = self
            .collection
            .find(doc! {"members": sub})
            .sort(doc! {"updated_at": -1})
            .await?;

        let chats = cursor.try_collect::<Vec<Chat>>().await?;

        Ok(chats)
    }

    /**
     * Find a chat by its id and the user sub
     * @param id: The id of the chat
     * @param sub: The user sub
     */
    pub async fn find_by_id_and_sub(&self, id: ChatId, sub: &UserSub) -> Result<Chat> {
        self.collection
            .find_one(doc! {
                "_id": id,
                "members": sub
            })
            .await?
            .ok_or(ChatError::NotFound(Some(id)))
    }

    /**
     * Find a chat id by its members
     * @param members: The members of the chat
     */
    pub async fn find_id_by_members(&self, members: [&UserSub; 2]) -> Result<ChatId> {
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

        Err(ChatError::NotFound(None))
    }
}
