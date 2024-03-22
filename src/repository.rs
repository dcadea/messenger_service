use log::{debug, error};
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
        debug!("Finding user with username: {}", username);
        let filter = doc! { "username": username };
        match self.collection.find_one(filter, None).await {
            Ok(result) => Ok(result.map(|doc| bson::from_document(doc).unwrap())),
            Err(e) => {
                error!("Failed to find user with username: {}. Error: {}", username, e);
                Err(e)
            }
        }
    }

    pub async fn insert(&self, user: User) -> Result<InsertOneResult, Error> {
        debug!("Inserting user with username: {}", user.username());
        let document = bson::to_document(&user).unwrap();
        match self.collection.insert_one(document, None).await {
            Ok(result) => Ok(result),
            Err(e) => {
                error!("Failed to insert user with username: {}. Error: {}", user.username(), e);
                Err(e)
            }
        }
    }

    pub async fn update(&self, user: User) -> Result<UpdateResult, Error> {
        debug!("Updating user with username: {}", user.username());
        let filter = doc! { "username": user.username() };
        let document = doc! { "$set": bson::to_document(&user).unwrap() };
        match self.collection.update_one(filter, document, None).await {
            Ok(result) => Ok(result),
            Err(e) => {
                error!("Failed to update user with username: {}. Error: {}", user.username(), e);
                Err(e)
            }
        }
    }

    pub async fn delete(&self, username: &str) -> Result<DeleteResult, Error> {
        debug!("Deleting user with username: {}", username);
        let filter = doc! { "username": username };
        match self.collection.delete_one(filter, None).await {
            Ok(result) => Ok(result),
            Err(e) => {
                error!("Failed to delete user with username: {}. Error: {}", username, e);
                Err(e)
            }
        }
    }
}