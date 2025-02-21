use futures::TryStreamExt;
use mongodb::Database;
use mongodb::bson::doc;
use mongodb::options::FindOneOptions;

use super::Sub;
use super::model::{Friends, User};
use crate::user;

const USERS_COLLECTION: &str = "users";

pub struct UserRepository {
    users_col: mongodb::Collection<User>,
    friends_col: mongodb::Collection<Friends>,
}

impl UserRepository {
    pub fn new(database: &Database) -> Self {
        Self {
            users_col: database.collection(USERS_COLLECTION),
            friends_col: database.collection(USERS_COLLECTION),
        }
    }
}

impl UserRepository {
    pub async fn insert(&self, user: &User) -> super::Result<()> {
        self.users_col.insert_one(user).await?;
        Ok(())
    }

    pub async fn find_by_sub(&self, sub: &Sub) -> super::Result<User> {
        let filter = doc! { "sub": sub };
        let result = self.users_col.find_one(filter).await?;
        result.ok_or(super::Error::NotFound(sub.to_owned()))
    }

    // search users by nickname excluding the logged user
    pub async fn search_by_nickname(
        &self,
        nickname: &str,
        logged_nickname: &str,
    ) -> super::Result<Vec<User>> {
        let filter = doc! {
            "$and": [
                { "nickname": { "$ne": logged_nickname } },
                { "nickname": { "$regex": nickname, "$options": "i" } },
            ]
        };

        let cursor = self.users_col.find(filter).await?;

        cursor.try_collect().await.map_err(super::Error::from)
    }

    pub async fn find_friends_by_sub(&self, sub: &user::Sub) -> super::Result<Vec<user::Sub>> {
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
            .ok_or(super::Error::NotFound(sub.to_owned()))
            .map(|f| f.friends)
    }

    pub async fn add_friend(&self, sub: &user::Sub, friend: &user::Sub) -> super::Result<()> {
        let filter = doc! { "sub": sub };
        let update = doc! { "$addToSet": { "friends": friend } };

        self.friends_col.update_one(filter, update).await?;

        Ok(())
    }

    pub async fn remove_friend(&self, sub: &user::Sub, friend: &user::Sub) -> super::Result<()> {
        let filter = doc! { "sub": sub };
        let update = doc! { "$pull": { "friends": friend } };

        self.friends_col.update_one(filter, update).await?;

        Ok(())
    }
}
