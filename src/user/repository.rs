use std::sync::Arc;

use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use mongodb::Database;

use crate::user::model::User;
use crate::result::Result;

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

    pub async fn insert(&self, user: &User) -> Result<Option<ObjectId>> {
        let inserted_id = self
            .collection
            .insert_one(user, None)
            .await?
            .inserted_id
            .as_object_id();

        Ok(inserted_id)
    }
}
