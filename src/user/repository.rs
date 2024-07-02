use futures::TryStreamExt;
use mongodb::bson::doc;
use mongodb::Database;

use crate::user::error::UserError;

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
        self.collection.insert_one(user).await?;
        Ok(())
    }

    pub async fn find_by_sub(&self, sub: &str) -> Result<User> {
        let filter = doc! { "sub": sub };
        let result = self.collection.find_one(filter).await?;
        result.ok_or(UserError::NotFound(sub.to_owned()))
    }

    pub async fn search_by_nickname(&self, nickname: &str) -> Result<Vec<User>> {
        let filter = doc! { "nickname":{
            "$regex": nickname,
            "$options": "i"
        }};

        let cursor = self.collection.find(filter).await?;

        cursor.try_collect().await.map_err(UserError::from)
    }
}

#[cfg(test)]
mod tests {
    // use crate::integration::mongo;
    // use crate::user::model::{User, UserSub};
    // use crate::user::repository::UserRepository;
    //
    // #[tokio::test]
    // async fn test_insert_user() {
    //     let mongo_config = MONGO_TEST_CONTAINER.get().await.config.clone();
    //     let database = mongo::init(&mongo_config).await.unwrap();
    //     let repository = UserRepository::new(&database);
    //
    //     let user = User::new(
    //         UserSub::from("valera"),
    //         "valera",
    //         "Valera",
    //         "valera.jpg",
    //         "valera@mail.test",
    //     );
    //     repository.insert(&user).await.unwrap();
    //
    //     let inserted = repository.find_by_sub("valera").await.unwrap();
    //     assert_eq!(inserted, inserted);
    // }
}
