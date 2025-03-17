use futures::TryStreamExt;
use mongodb::Database;
use mongodb::bson::doc;
use mongodb::options::FindOneOptions;
use user::Sub;

use super::model::{Contacts, User};
use crate::user;

const USERS_COLLECTION: &str = "users";

pub struct UserRepository {
    users_col: mongodb::Collection<User>,
    contacts_col: mongodb::Collection<Contacts>,
}

impl UserRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            users_col: db.collection(USERS_COLLECTION),
            contacts_col: db.collection(USERS_COLLECTION),
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

    pub async fn find_contacts_for_sub(&self, sub: &Sub) -> super::Result<Vec<Sub>> {
        let filter = doc! { "sub": sub };
        let projection = FindOneOptions::builder()
            .projection(doc! { "contacts": 1 })
            .build();

        let contacts = self
            .contacts_col
            .find_one(filter)
            .with_options(projection)
            .await?;

        contacts
            .ok_or(super::Error::NotFound(sub.to_owned()))
            .map(|f| f.contacts)
    }

    // TODO: revisit this
    pub async fn add_contact(&self, sub: &Sub, contact: &Sub) -> super::Result<()> {
        let filter = doc! { "sub": sub };
        let update = doc! { "$addToSet": { "contacts": contact } };

        self.contacts_col.update_one(filter, update).await?;

        Ok(())
    }

    // TODO: revisit this
    pub async fn remove_contact(&self, sub: &Sub, contact: &Sub) -> super::Result<()> {
        let filter = doc! { "sub": { "$in": [sub, contact] } };
        let update = doc! { "$pull": { "contacts": { "$in": [sub, contact] } } };

        self.contacts_col.update_many(filter, update).await?;

        Ok(())
    }
}
