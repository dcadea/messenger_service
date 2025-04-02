use futures::TryStreamExt;
use mongodb::Database;
use mongodb::bson::doc;
use user::Sub;

use super::model::User;
use crate::user;

const USERS_COLLECTION: &str = "users";

#[async_trait::async_trait]
pub trait UserRepository {
    async fn insert(&self, user: &User) -> super::Result<()>;

    async fn find_by_sub(&self, sub: &Sub) -> super::Result<User>;

    // search users by nickname excluding the authenticated user
    async fn search_by_nickname(
        &self,
        nickname: &str,
        auth_nickname: &str,
    ) -> super::Result<Vec<User>>;
}

pub struct MongoUserRepository {
    col: mongodb::Collection<User>,
}

impl MongoUserRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            col: db.collection(USERS_COLLECTION),
        }
    }
}

#[async_trait::async_trait]
impl UserRepository for MongoUserRepository {
    async fn insert(&self, user: &User) -> super::Result<()> {
        self.col.insert_one(user).await?;
        Ok(())
    }

    async fn find_by_sub(&self, sub: &Sub) -> super::Result<User> {
        let filter = doc! { "sub": sub };
        let result = self.col.find_one(filter).await?;
        result.ok_or(super::Error::NotFound(sub.to_owned()))
    }

    // search users by nickname excluding the authenticated user
    async fn search_by_nickname(
        &self,
        nickname: &str,
        auth_nickname: &str,
    ) -> super::Result<Vec<User>> {
        let filter = doc! {
            "$and": [
                { "nickname": { "$ne": auth_nickname } },
                { "nickname": { "$regex": nickname, "$options": "i" } },
            ]
        };

        let cursor = self.col.find(filter).await?;

        cursor.try_collect().await.map_err(super::Error::from)
    }
}
