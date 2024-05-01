use std::sync::Arc;

use mongodb::bson::doc;
use mongodb::Database;

use crate::result::Result;
use crate::user::model::User;

pub struct UserRepository {
    collection: mongodb::Collection<User>,
}

impl UserRepository {
    pub fn new(database: &Database) -> Arc<Self> {
        let collection = database.collection("users");
        Self { collection }.into()
    }
}

impl UserRepository {
    pub(super) async fn find_one(&self, username: &str) -> Option<User> {
        let filter = doc! { "username": username };

        if let Ok(u) = self.collection.find_one(filter, None).await {
            return u;
        }

        None
    }

    pub(super) async fn insert(&self, user: &User) -> Result<()> {
        self.collection.insert_one(user, None).await?;
        Ok(())
    }
}
