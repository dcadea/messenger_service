use futures::TryStreamExt;
use mongodb::{Database, bson::doc};

use crate::user;

use super::model::Contact;

const CONTACTS_COLLECTION: &str = "contacts";

#[async_trait::async_trait]
pub trait ContactRepository {
    async fn find(&self, sub1: &user::Sub, sub2: &user::Sub) -> super::Result<Option<Contact>>;

    async fn find_by_sub(&self, sub: &user::Sub) -> super::Result<Vec<Contact>>;

    async fn add(&self, contact: &Contact) -> super::Result<()>;

    async fn delete(&self, from: &user::Sub, contact: &user::Sub) -> super::Result<()>;

    async fn exists(&self, sub1: &user::Sub, sub2: &user::Sub) -> super::Result<bool>;
}

pub struct MongoContactRepository {
    col: mongodb::Collection<Contact>,
}

impl MongoContactRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            col: db.collection(CONTACTS_COLLECTION),
        }
    }
}

#[async_trait::async_trait]
impl ContactRepository for MongoContactRepository {
    async fn find(&self, sub1: &user::Sub, sub2: &user::Sub) -> super::Result<Option<Contact>> {
        let filter = doc! { "$or": [ {"sub1": sub1, "sub2": sub2}, {"sub2": sub1, "sub1": sub2} ] };

        self.col.find_one(filter).await.map_err(super::Error::from)
    }

    async fn find_by_sub(&self, sub: &user::Sub) -> super::Result<Vec<Contact>> {
        let filter = doc! { "$or": [ {"sub1": sub}, {"sub2": sub} ] };

        let cursor = self.col.find(filter).await?;

        cursor.try_collect().await.map_err(super::Error::from)
    }

    async fn add(&self, c: &Contact) -> super::Result<()> {
        assert_ne!(c.sub1, c.sub2);

        self.col.insert_one(c).await?;

        Ok(())
    }

    async fn delete(&self, me: &user::Sub, you: &user::Sub) -> super::Result<()> {
        let filter = doc! { "$or": [ {"sub1": me, "sub2": you}, {"sub2": me, "sub1": you} ] };

        self.col.delete_one(filter).await?;

        Ok(())
    }

    async fn exists(&self, sub1: &user::Sub, sub2: &user::Sub) -> super::Result<bool> {
        let filter = doc! {
            "$or": [
                {"sub1": sub1, "sub2": sub2},
                {"sub2": sub1, "sub1": sub2}
            ]
        };

        let result = self.col.find_one(filter).await?;
        Ok(result.is_some())
    }
}
