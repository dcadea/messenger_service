use mongodb::bson::{doc, Document};
use mongodb::Database;

use crate::result::Result;
use crate::user::model::User;

pub struct UserRepository {
    collection: mongodb::Collection<User>,
}

impl UserRepository {
    pub fn new(database: &Database) -> Self {
        Self {
            collection: database.collection("users"),
        }
    }
}

impl UserRepository {
    pub(super) async fn insert(&self, user: &User) -> Result<()> {
        self.collection.insert_one(user, None).await?;
        Ok(())
    }

    pub(super) async fn find_by_sub(&self, sub: &str) -> Option<User> {
        self.find(doc! { "sub": sub }).await
    }

    async fn find(&self, filter: Document) -> Option<User> {
        self.collection.find_one(filter, None).await.ok().flatten()
    }
}
