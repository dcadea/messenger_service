use std::sync::Arc;

use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use mongodb::error::Error;
use mongodb::Database;

use crate::user::model::User;

pub struct UserRepository {
    collection: mongodb::Collection<User>,
}

impl UserRepository {
    pub fn new(database: &Database) -> Arc<Self> {
        let collection = database.collection("users");
        Self { collection }.into()
    }

    pub async fn find_one(&self, username: &str) -> Option<User> {
        let filter = doc! { "username": username };

        if let Ok(u) = self.collection.find_one(filter, None).await {
            return u;
        }

        None
    }

    pub async fn insert(&self, user: &User) -> Result<Option<ObjectId>, Error> {
        self.collection
            .insert_one(user, None)
            .await
            .map(|r| r.inserted_id.as_object_id())
    }

    pub async fn update(&self, user: User) -> Result<bool, Error> {
        let filter = doc! { "username": user.username() };
        let document = doc! { "$set": user };

        self.collection
            .update_one(filter, document, None)
            .await
            .map(|r| r.modified_count > 0)
    }

    pub async fn delete(&self, username: &str) -> Result<bool, Error> {
        let filter = doc! { "username": username };

        self.collection
            .delete_one(filter, None)
            .await
            .map(|r| r.deleted_count > 0)
    }
}
