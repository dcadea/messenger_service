use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::Database;
use mongodb::bson::doc;

use super::{Id, model::Message};
use crate::{message, talk};

const MESSAGES_COLLECTION: &str = "messages";

#[async_trait]
pub trait MessageRepository {
    async fn insert(&self, msg: &Message) -> super::Result<()>;

    async fn insert_many(&self, msgs: &[Message]) -> super::Result<()>;

    async fn find_by_id(&self, id: &Id) -> super::Result<Message>;

    async fn find_by_talk_id(&self, talk_id: &talk::Id) -> super::Result<Vec<Message>>;

    async fn find_by_talk_id_limited(
        &self,
        talk_id: &talk::Id,
        limit: i64,
    ) -> super::Result<Vec<Message>>;

    async fn find_by_talk_id_before(
        &self,
        talk_id: &talk::Id,
        before: i64,
    ) -> super::Result<Vec<Message>>;

    async fn find_by_talk_id_limited_before(
        &self,
        talk_id: &talk::Id,
        limit: i64,
        before: i64,
    ) -> super::Result<Vec<Message>>;

    async fn find_most_recent(&self, talk_id: &talk::Id) -> super::Result<Option<Message>>;

    async fn update(&self, id: &Id, text: &str) -> super::Result<bool>;

    async fn delete(&self, id: &Id) -> super::Result<bool>;

    async fn delete_by_talk_id(&self, talk_id: &talk::Id) -> super::Result<u64>;

    async fn mark_as_seen(&self, ids: &[Id]) -> super::Result<u64>;
}

#[derive(Clone)]
pub struct MongoMessageRepository {
    col: mongodb::Collection<Message>,
}

impl MongoMessageRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            col: db.collection(MESSAGES_COLLECTION),
        }
    }
}

#[async_trait]
impl MessageRepository for MongoMessageRepository {
    async fn insert(&self, msg: &Message) -> super::Result<()> {
        let result = self.col.insert_one(msg).await?;
        result
            .inserted_id
            .as_object_id()
            .ok_or(super::Error::IdNotPresent)?;
        Ok(())
    }

    async fn insert_many(&self, msgs: &[Message]) -> super::Result<()> {
        self.col.insert_many(msgs).await?;

        Ok(())
    }

    async fn find_by_id(&self, id: &Id) -> super::Result<Message> {
        self.col
            .find_one(doc! {"_id": id})
            .await?
            .ok_or(message::Error::NotFound(Some(id.to_owned())))
    }

    async fn find_by_talk_id(&self, talk_id: &talk::Id) -> super::Result<Vec<Message>> {
        let cursor = self
            .col
            .find(doc! {"talk_id": talk_id})
            .sort(doc! {"timestamp": -1})
            .await?;

        let messages = cursor.try_collect::<Vec<Message>>().await?;

        Ok(messages)
    }

    async fn find_by_talk_id_limited(
        &self,
        talk_id: &talk::Id,
        limit: i64,
    ) -> super::Result<Vec<Message>> {
        let cursor = self
            .col
            .find(doc! {"talk_id": talk_id})
            .sort(doc! {"timestamp": -1})
            .limit(limit)
            .await?;

        let messages = cursor.try_collect::<Vec<Message>>().await?;

        Ok(messages)
    }

    async fn find_by_talk_id_before(
        &self,
        talk_id: &talk::Id,
        before: i64,
    ) -> super::Result<Vec<Message>> {
        let cursor = self
            .col
            .find(doc! {
                "talk_id": talk_id,
                "timestamp": {"$lt": before}
            })
            .sort(doc! {"timestamp": -1})
            .await?;

        let msgs = cursor.try_collect::<Vec<Message>>().await?;

        Ok(msgs)
    }

    async fn find_by_talk_id_limited_before(
        &self,
        talk_id: &talk::Id,
        limit: i64,
        before: i64,
    ) -> super::Result<Vec<Message>> {
        let cursor = self
            .col
            .find(doc! {
                "talk_id": talk_id,
                "timestamp": {"$lt": before}
            })
            .sort(doc! {"timestamp": -1})
            .limit(limit)
            .await?;

        let msgs = cursor.try_collect::<Vec<Message>>().await?;

        Ok(msgs)
    }

    async fn find_most_recent(&self, talk_id: &talk::Id) -> super::Result<Option<Message>> {
        let mut cursor = self
            .col
            .find(doc! {"talk_id": talk_id})
            .sort(doc! {"timestamp": -1})
            .limit(1)
            .await?;

        let msg = cursor.try_next().await?;

        Ok(msg)
    }

    async fn update(&self, id: &Id, text: &str) -> super::Result<bool> {
        let res = self
            .col
            .update_one(doc! {"_id": id}, doc! {"$set": {"text": text}})
            .await?;

        Ok(res.modified_count > 0)
    }

    async fn delete(&self, id: &Id) -> super::Result<bool> {
        let res = self.col.delete_one(doc! {"_id": id}).await?;

        Ok(res.deleted_count > 0)
    }

    async fn delete_by_talk_id(&self, talk_id: &talk::Id) -> super::Result<u64> {
        let res = self.col.delete_many(doc! {"talk_id": talk_id}).await?;

        Ok(res.deleted_count)
    }

    async fn mark_as_seen(&self, ids: &[Id]) -> super::Result<u64> {
        let res = self
            .col
            .update_many(
                doc! {"_id": {"$in": ids}, "seen": false},
                doc! {"$set": {"seen": true}},
            )
            .await?;
        Ok(res.modified_count)
    }
}

#[cfg(test)]
mod test {
    use testcontainers_modules::{mongo::Mongo, testcontainers::runners::AsyncRunner};

    use crate::{integration::db, user::Sub};

    use super::*;

    #[tokio::test]
    async fn should_insert() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::Config::test(&node).await.connect();
        let repo = MongoMessageRepository::new(&db);

        let expected = Message::new(talk::Id::random(), Sub::from("jora"), "Hello, world!");

        repo.insert(&expected).await.unwrap();
        let actual = repo.find_by_id(expected.id()).await.unwrap();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn should_insert_many() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::Config::test(&node).await.connect();
        let repo = MongoMessageRepository::new(&db);

        let m1 = Message::new(talk::Id::random(), Sub::from("jora"), "Hello, world!");
        let m2 = Message::new(talk::Id::random(), Sub::from("val"), "Goodbye, world!");

        repo.insert_many(&[m1.clone(), m2.clone()]).await.unwrap();
        let actual1 = repo.find_by_id(m1.id()).await.unwrap();
        let actual2 = repo.find_by_id(m2.id()).await.unwrap();

        assert_eq!([actual1, actual2], [m1, m2]);
    }

    #[tokio::test]
    async fn should_not_insert_many() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::Config::test(&node).await.connect();
        let repo = MongoMessageRepository::new(&db);

        let m1 = Message::new(talk::Id::random(), Sub::from("jora"), "Hello, world!");
        let m2 = Message::new(talk::Id::random(), Sub::from("val"), "Goodbye, world!");
        let m3 = m2.clone();

        let res = repo
            .insert_many(&[m1.clone(), m2.clone(), m3.clone()])
            .await;

        assert!(res.is_err())
    }

    #[tokio::test]
    async fn should_find_by_talk_id() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::Config::test(&node).await.connect();
        let repo = MongoMessageRepository::new(&db);

        let talk_id = talk::Id::random();
        let m1 = Message::new(talk_id.clone(), Sub::from("jora"), "Hello, world!");
        let m2 = Message::new(talk_id.clone(), Sub::from("val"), "Goodbye, world!");
        let m3 = Message::new(talk::Id::random(), Sub::from("radu"), "What's up?");

        let expected = vec![m1.clone(), m2.clone()];

        repo.insert_many(&[m1, m2, m3]).await.unwrap();
        let actual = repo.find_by_talk_id(&talk_id).await.unwrap();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn should_find_by_talk_id_limited() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::Config::test(&node).await.connect();
        let repo = MongoMessageRepository::new(&db);

        let talk_id = talk::Id::random();
        let m1 = Message::new(talk_id.clone(), Sub::from("jora"), "Hello, world!");
        let m2 = Message::new(talk_id.clone(), Sub::from("val"), "Goodbye, world!");
        let m3 = Message::new(talk_id.clone(), Sub::from("radu"), "What's up?");
        let m4 = Message::new(talk_id.clone(), Sub::from("igor"), "Not mutch");

        let expected = vec![m1.clone(), m2.clone()];

        repo.insert_many(&[m1, m2, m3, m4]).await.unwrap();
        let actual = repo.find_by_talk_id_limited(&talk_id, 2).await.unwrap();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn should_find_by_talk_id_before() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::Config::test(&node).await.connect();
        let repo = MongoMessageRepository::new(&db);

        let talk_id = talk::Id::random();
        let now = chrono::Utc::now().timestamp();
        let mut m1 = Message::new(talk_id.clone(), Sub::from("jora"), "Hello, world!");
        m1.set_timestamp(now - 3000);
        let mut m2 = Message::new(talk_id.clone(), Sub::from("val"), "Goodbye, world!");
        m2.set_timestamp(now - 2000);
        let mut m3 = Message::new(talk_id.clone(), Sub::from("radu"), "What's up?");
        m3.set_timestamp(now - 1000);
        let m4 = Message::new(talk_id.clone(), Sub::from("igor"), "Not mutch");

        let expected = vec![m2.clone(), m1.clone()];
        let before = m3.timestamp();

        repo.insert_many(&[m1, m2, m3, m4]).await.unwrap();
        let actual = repo.find_by_talk_id_before(&talk_id, before).await.unwrap();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn should_find_by_talk_id_limited_before() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::Config::test(&node).await.connect();
        let repo = MongoMessageRepository::new(&db);

        let talk_id = talk::Id::random();
        let now = chrono::Utc::now().timestamp();
        let mut m1 = Message::new(talk_id.clone(), Sub::from("jora"), "Hello, world!");
        m1.set_timestamp(now - 4000);
        let mut m2 = Message::new(talk_id.clone(), Sub::from("val"), "Goodbye, world!");
        m2.set_timestamp(now - 3000);
        let mut m3 = Message::new(talk_id.clone(), Sub::from("radu"), "What's up?");
        m3.set_timestamp(now - 2000);
        let mut m4 = Message::new(talk_id.clone(), Sub::from("igor"), "Not mutch");
        m4.set_timestamp(now - 1000);
        let m5 = Message::new(talk_id.clone(), Sub::from("igor"), "Not mutch");

        let expected = vec![m3.clone(), m2.clone()];
        let before = m4.timestamp();

        repo.insert_many(&[m1, m2, m3, m4, m5]).await.unwrap();
        let actual = repo
            .find_by_talk_id_limited_before(&talk_id, 2, before)
            .await
            .unwrap();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn should_find_most_recent() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::Config::test(&node).await.connect();
        let repo = MongoMessageRepository::new(&db);

        let talk_id = talk::Id::random();
        let now = chrono::Utc::now().timestamp();
        let mut m1 = Message::new(talk_id.clone(), Sub::from("jora"), "Hello, world!");
        m1.set_timestamp(now - 3000);
        let mut m2 = Message::new(talk_id.clone(), Sub::from("val"), "Goodbye, world!");
        m2.set_timestamp(now - 2000);
        let mut m3 = Message::new(talk_id.clone(), Sub::from("radu"), "What's up?");
        m3.set_timestamp(now - 1000);
        let m4 = Message::new(talk::Id::random(), Sub::from("igor"), "Not mutch");

        let expected = m3.clone();

        repo.insert_many(&[m1, m2, m3, m4]).await.unwrap();
        let actual = repo.find_most_recent(&talk_id).await.unwrap().unwrap();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn should_not_find_most_recent() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::Config::test(&node).await.connect();
        let repo = MongoMessageRepository::new(&db);

        let m = Message::new(talk::Id::random(), Sub::from("jora"), "Hello, world!");

        repo.insert(&m).await.unwrap();
        let actual = repo.find_most_recent(&talk::Id::random()).await.unwrap();

        assert!(actual.is_none());
    }

    #[tokio::test]
    async fn should_update_text() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::Config::test(&node).await.connect();
        let repo = MongoMessageRepository::new(&db);

        let m = Message::new(talk::Id::random(), Sub::from("jora"), "Hello, world!");

        repo.insert(&m).await.unwrap();

        let updated = repo.update(m.id(), "Goodbye, world!").await.unwrap();
        assert!(updated);

        let actual = repo.find_by_id(m.id()).await.unwrap();

        assert_eq!(actual.text(), "Goodbye, world!");
    }

    #[tokio::test]
    async fn should_not_update_text() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::Config::test(&node).await.connect();
        let repo = MongoMessageRepository::new(&db);

        let m = Message::new(talk::Id::random(), Sub::from("jora"), "Hello, world!");

        repo.insert(&m).await.unwrap();

        let updated = repo
            .update(&message::Id::random(), "Goodbye, world!")
            .await
            .unwrap();
        assert!(!updated);

        let actual = repo.find_by_id(m.id()).await.unwrap();

        assert_eq!(actual.text(), "Hello, world!");
    }

    #[tokio::test]
    async fn should_delete() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::Config::test(&node).await.connect();
        let repo = MongoMessageRepository::new(&db);

        let m = Message::new(talk::Id::random(), Sub::from("jora"), "Hello, world!");

        repo.insert(&m).await.unwrap();
        assert!(repo.find_by_id(m.id()).await.is_ok());

        let deleted = repo.delete(m.id()).await.unwrap();
        assert!(deleted);

        let actual = repo.find_by_id(m.id()).await.unwrap_err();

        assert!(matches!(actual, message::Error::NotFound(Some(id)) if id.eq(m.id())));
    }

    #[tokio::test]
    async fn should_not_delete() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::Config::test(&node).await.connect();
        let repo = MongoMessageRepository::new(&db);

        let m = Message::new(talk::Id::random(), Sub::from("jora"), "Hello, world!");

        repo.insert(&m).await.unwrap();
        assert!(repo.find_by_id(m.id()).await.is_ok());

        let deleted = repo.delete(&message::Id::random()).await.unwrap();
        assert!(!deleted);

        assert!(repo.find_by_id(m.id()).await.is_ok());
    }

    #[tokio::test]
    async fn should_delete_by_talk_id() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::Config::test(&node).await.connect();
        let repo = MongoMessageRepository::new(&db);

        let talk_id = talk::Id::random();
        let m1 = Message::new(talk_id.clone(), Sub::from("jora"), "Hello, world!");
        let m2 = Message::new(talk::Id::random(), Sub::from("val"), "Goodbye!");
        let m3 = Message::new(talk_id.clone(), Sub::from("radu"), "What's up?");
        let m4 = Message::new(talk::Id::random(), Sub::from("igor"), "Not mutch");

        repo.insert_many(&[m1, m2, m3, m4]).await.unwrap();
        let delete_count = repo.delete_by_talk_id(&talk_id).await.unwrap();

        assert_eq!(delete_count, 2);
    }

    #[tokio::test]
    async fn should_mark_as_seen() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::Config::test(&node).await.connect();
        let repo = MongoMessageRepository::new(&db);

        let talk_id = talk::Id::random();
        let m1 = Message::new(talk_id.clone(), Sub::from("jora"), "Hello, world!");
        let m2 = Message::new(talk::Id::random(), Sub::from("val"), "Goodbye!");
        let m3 = Message::new(talk_id.clone(), Sub::from("radu"), "What's up?");
        let mut m4 = Message::new(talk_id.clone(), Sub::from("igor"), "Not mutch");
        m4.set_seen(true);

        let ids = [
            m1.id().clone(),
            m2.id().clone(),
            m3.id().clone(),
            m4.id().clone(),
        ];

        repo.insert_many(&[m1, m2, m3, m4]).await.unwrap();
        let seen_qty = repo.mark_as_seen(&ids).await.unwrap();

        assert_eq!(seen_qty, 3);
    }
}
