use futures::stream::TryStreamExt;
use mongodb::bson::doc;
use mongodb::options::FindOptions;

use crate::user::model::UserSub;

use super::error::ChatError;
use super::model::{Chat, ChatId, Members};
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
    pub async fn insert(&self, chat: &Chat) -> Result<ChatId> {
        let result = self.collection.insert_one(chat, None).await?;
        if let Some(id) = result.inserted_id.as_object_id() {
            return Ok(id.to_owned());
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
        let filter = doc! { "_id": id };
        let update = doc! {"$set": {
            "last_message": text,
            "updated_at": chrono::Utc::now().timestamp(),
        }};
        self.collection.update_one(filter, update, None).await?;
        Ok(())
    }

    /**
     * Find a chat where the user sub is a member
     * @param sub: The user sub
     */
    pub async fn find_by_sub(&self, sub: &UserSub) -> Result<Vec<Chat>> {
        let filter = doc! {
            "$or": [
                { "members.me": sub },
                { "members.you": sub },
            ]
        };

        let find_options = FindOptions::builder().sort(doc! {"updated_at": -1}).build();

        let cursor = self.collection.find(Some(filter), find_options).await?;
        cursor.try_collect().await.map_err(ChatError::from)
    }

    /**
     * Find a chat by its id and the user sub
     * @param id: The id of the chat
     * @param sub: The user sub
     */
    pub async fn find_by_id_and_sub(&self, id: ChatId, sub: &UserSub) -> Result<Chat> {
        let filter = doc! {
            "_id": id,
            "$or": [
                { "members.me": sub },
                { "members.you": sub },
            ]
        };

        let result = self.collection.find_one(Some(filter), None).await?;
        result.ok_or(ChatError::NotFound(Some(id)))
    }

    /**
     * Find a chat id by its members
     * @param members: The members of the chat
     */
    pub async fn find_id_by_members(&self, members: &Members) -> Result<ChatId> {
        let me = &members.me;
        let you = &members.you;

        let filter = doc! {
            "$or": [
                { "members.me": me, "members.you": you },
                { "members.me": you, "members.you": me },
            ]
        };

        let result = self.collection.find_one(Some(filter), None).await?;
        if let Some(chat) = result {
            if let Some(id) = chat.id {
                return Ok(id);
            }
        }

        Err(ChatError::NotFound(None))
    }
}
