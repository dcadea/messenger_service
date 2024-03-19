use mongodb::{bson, Database};
use mongodb::bson::{doc, Document};
use mongodb::error::Error;
use mongodb::results::{DeleteResult, InsertOneResult, UpdateResult};

use crate::models::User;

#[derive(Clone)]
pub struct UserRepository {
    collection: mongodb::Collection<Document>,
}

impl UserRepository {
    pub fn new(database: Database) -> Self {
        let collection = database.collection("users");
        Self { collection }
    }

    pub async fn find_one(&self, username: &str) -> Result<Option<User>, Error> {
        let filter = doc! { "username": username };
        self.collection.find_one(filter, None).await.map(|result| {
            result.map(|doc| bson::from_document(doc).unwrap())
        })
    }

    pub async fn insert(&self, user: User) -> Result<InsertOneResult, Error> {
        let document = bson::to_document(&user).unwrap();
        self.collection.insert_one(document, None).await
    }

    pub async fn update(&self, user: User) -> Result<UpdateResult, Error> {
        let filter = doc! { "username": user.username() };
        let document = doc! { "$set": bson::to_document(&user).unwrap() };
        self.collection.update_one(filter, document, None).await
    }

    pub async fn delete(&self, username: &str) -> Result<DeleteResult, Error> {
        let filter = doc! { "username": username };
        self.collection.delete_one(filter, None).await
    }
}