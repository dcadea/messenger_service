use mongodb::bson::doc;
use mongodb::Database;

use super::model::User;
use super::Result;

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
    pub async fn insert(&self, user: &User) -> Result<()> {
        self.collection.insert_one(user, None).await?;
        Ok(())
    }

    pub async fn find_by_sub(&self, sub: &str) -> Option<User> {
        let filter = doc! { "sub": sub };
        self.collection.find_one(filter, None).await.ok().flatten()
    }
}
