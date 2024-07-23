use futures::TryStreamExt;
use mongodb::bson::doc;
use mongodb::options::FindOneOptions;
use mongodb::Database;

use super::model::{Friends, Sub, User};
use super::Result;
use crate::user;

pub struct UserRepository {
    users_col: mongodb::Collection<User>,
    friends_col: mongodb::Collection<Friends>,
}

impl UserRepository {
    pub fn new(database: &Database) -> Self {
        Self {
            users_col: database.collection("users"),
            friends_col: database.collection("users"),
        }
    }
}

impl UserRepository {
    pub async fn insert(&self, user: &User) -> Result<()> {
        self.users_col.insert_one(user).await?;
        Ok(())
    }

    pub async fn find_by_sub(&self, sub: &Sub) -> Result<User> {
        let filter = doc! { "sub": sub };
        let result = self.users_col.find_one(filter).await?;
        result.ok_or(user::Error::NotFound(sub.to_owned()))
    }

    pub async fn search_by_nickname(&self, nickname: &str) -> Result<Vec<User>> {
        let filter = doc! { "nickname":{
            "$regex": nickname,
            "$options": "i"
        }};

        let cursor = self.users_col.find(filter).await?;

        cursor.try_collect().await.map_err(user::Error::from)
    }

    pub async fn add_friend(&self, sub: &Sub, friend: &Sub) -> Result<()> {
        let filter = doc! { "sub": sub };
        let update = doc! { "$push": { "friends": friend } };

        self.users_col.update_one(filter, update).await?;
        Ok(())
    }

    pub async fn find_friends_by_sub(&self, sub: &Sub) -> Result<Vec<Sub>> {
        let filter = doc! { "sub": sub };
        let projection = FindOneOptions::builder()
            .projection(doc! { "friends": 1 })
            .build();

        let friends = self
            .friends_col
            .find_one(filter)
            .with_options(projection)
            .await?;

        friends
            .ok_or(user::Error::NotFound(sub.clone()))
            .map(|f| f.friends)
    }
}
