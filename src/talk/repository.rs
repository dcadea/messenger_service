use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::bson::doc;

use super::model::Talk;
use crate::{message::model::LastMessage, talk, user::Sub};

const TALKS_COLLECTION: &str = "talks";

#[async_trait]
pub trait TalkRepository {
    async fn find_by_id(&self, id: &talk::Id) -> super::Result<Talk>;

    async fn find_by_sub_and_kind(&self, sub: &Sub, kind: &talk::Kind) -> super::Result<Vec<Talk>>;

    async fn find_by_id_and_sub(&self, id: &talk::Id, sub: &Sub) -> super::Result<Talk>;

    async fn create(&self, talk: &Talk) -> super::Result<()>;

    async fn delete(&self, id: &talk::Id) -> super::Result<bool>;

    async fn exists(&self, members: &[Sub; 2]) -> super::Result<bool>;

    async fn update_last_message(
        &self,
        id: &talk::Id,
        msg: Option<&LastMessage>,
    ) -> super::Result<()>;

    async fn mark_as_seen(&self, id: &talk::Id) -> super::Result<()>;
}

#[derive(Clone)]
pub struct MongoTalkRepository {
    col: mongodb::Collection<Talk>,
}

impl MongoTalkRepository {
    pub fn new(db: &mongodb::Database) -> Self {
        Self {
            col: db.collection(TALKS_COLLECTION),
        }
    }
}

#[async_trait]
impl TalkRepository for MongoTalkRepository {
    async fn find_by_id(&self, id: &talk::Id) -> super::Result<Talk> {
        let talk = self.col.find_one(doc! { "_id": id }).await?;

        talk.ok_or(talk::Error::NotFound(Some(id.to_owned())))
    }

    async fn find_by_sub_and_kind(&self, sub: &Sub, kind: &talk::Kind) -> super::Result<Vec<Talk>> {
        let cursor = self
            .col
            .find(doc! {
                "kind": kind.as_str(),
                "details.members": sub,
            })
            .sort(doc! {"last_message.timestamp": -1})
            .await?;

        let talks: Vec<Talk> = cursor.try_collect().await?;

        Ok(talks)
    }

    async fn find_by_id_and_sub(&self, id: &talk::Id, sub: &Sub) -> super::Result<Talk> {
        let talk = self
            .col
            .find_one(doc! {
                "_id": id,
                "details.members": sub
            })
            .await?;

        talk.ok_or(talk::Error::NotFound(Some(id.to_owned())))
    }

    async fn create(&self, talk: &Talk) -> super::Result<()> {
        self.col.insert_one(talk).await?;

        Ok(())
    }

    async fn delete(&self, id: &talk::Id) -> super::Result<bool> {
        let res = self.col.delete_one(doc! {"_id": id}).await?;

        Ok(res.deleted_count > 0)
    }

    async fn exists(&self, members: &[Sub; 2]) -> super::Result<bool> {
        let count = self
            .col
            .count_documents(doc! { "details.members": { "$all": members.to_vec() } })
            .await?;

        Ok(count > 0)
    }

    async fn update_last_message(
        &self,
        id: &talk::Id,
        msg: Option<&LastMessage>,
    ) -> super::Result<()> {
        self.col
            .update_one(
                doc! { "_id": id },
                doc! {"$set": {
                    "last_message": msg,
                }},
            )
            .await?;
        Ok(())
    }

    async fn mark_as_seen(&self, id: &talk::Id) -> super::Result<()> {
        self.col
            .update_one(
                doc! {
                    "$and": [
                        {"_id": id },
                        { "last_message.seen": { "$exists": true }}
                    ]
                },
                doc! {"$set": {
                    "last_message.seen": true,
                }},
            )
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use testcontainers_modules::{mongo::Mongo, testcontainers::runners::AsyncRunner};

    use crate::{
        integration::db,
        message::{self, model::LastMessage},
        talk::{
            self,
            model::{Details, Talk},
        },
        user::Sub,
    };

    use super::{MongoTalkRepository, TalkRepository};

    #[tokio::test]
    async fn should_find_by_id() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::mongo::Config::test(&node).await.connect();
        let repo = MongoTalkRepository::new(&db);

        let expected = Talk::from(Details::Chat {
            members: [Sub::from("jora"), Sub::from("valera")],
        });
        repo.create(&expected).await.unwrap();

        let actual = repo.find_by_id(expected.id()).await.unwrap();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn should_not_find_by_id() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::mongo::Config::test(&node).await.connect();
        let repo = MongoTalkRepository::new(&db);

        let talk_id = talk::Id::random();
        let actual = repo.find_by_id(&talk_id).await.unwrap_err();

        assert!(matches!(actual, talk::Error::NotFound(Some(id)) if id.eq(&talk_id)));
    }

    #[tokio::test]
    async fn should_find_by_sub_and_kind() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::mongo::Config::test(&node).await.connect();
        let repo = MongoTalkRepository::new(&db);

        let t1 = &Talk::from(Details::Chat {
            members: [Sub::from("jora"), Sub::from("valera")],
        });
        let t2 = &Talk::from(Details::Chat {
            members: [Sub::from("jora"), Sub::from("igor")],
        });
        let t3 = &Talk::from(Details::Chat {
            members: [Sub::from("radu"), Sub::from("igor")],
        });
        let t4 = &Talk::from(Details::Group {
            name: "g1".into(),
            owner: Sub::from("radu"),
            members: vec![Sub::from("jora"), Sub::from("radu"), Sub::from("igor")],
        });

        tokio::try_join!(
            repo.create(t1),
            repo.create(t2),
            repo.create(t3),
            repo.create(t4),
        )
        .unwrap();

        let mut expected = vec![t1, t2].into_iter();

        let actual = repo
            .find_by_sub_and_kind(&Sub::from("jora"), &talk::Kind::Chat)
            .await
            .unwrap();

        assert_eq!(expected.len(), actual.len());
        assert!(expected.all(|t| actual.contains(t)));
    }

    #[tokio::test]
    async fn should_not_find_by_sub_and_kind() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::mongo::Config::test(&node).await.connect();
        let repo = MongoTalkRepository::new(&db);

        let t1 = &Talk::from(Details::Chat {
            members: [Sub::from("jora"), Sub::from("valera")],
        });
        let t2 = &Talk::from(Details::Group {
            name: "g1".into(),
            owner: Sub::from("radu"),
            members: vec![Sub::from("jora"), Sub::from("radu"), Sub::from("igor")],
        });

        tokio::try_join!(repo.create(t1), repo.create(t2),).unwrap();

        let actual = repo
            .find_by_sub_and_kind(&Sub::from("radu"), &talk::Kind::Chat)
            .await
            .unwrap();

        assert!(actual.is_empty());
    }

    #[tokio::test]
    async fn should_find_chat_by_id_and_sub1() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::mongo::Config::test(&node).await.connect();
        let repo = MongoTalkRepository::new(&db);

        let expected = Talk::from(Details::Chat {
            members: [Sub::from("jora"), Sub::from("valera")],
        });
        repo.create(&expected).await.unwrap();

        let actual = repo
            .find_by_id_and_sub(expected.id(), &Sub::from("jora"))
            .await
            .unwrap();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn should_find_chat_by_id_and_sub2() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::mongo::Config::test(&node).await.connect();
        let repo = MongoTalkRepository::new(&db);

        let expected = Talk::from(Details::Chat {
            members: [Sub::from("jora"), Sub::from("valera")],
        });
        repo.create(&expected).await.unwrap();

        let actual = repo
            .find_by_id_and_sub(expected.id(), &Sub::from("valera"))
            .await
            .unwrap();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn should_find_group_by_id_and_sub1() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::mongo::Config::test(&node).await.connect();
        let repo = MongoTalkRepository::new(&db);

        let expected = Talk::from(Details::Group {
            name: "g1".into(),
            owner: Sub::from("radu"),
            members: vec![Sub::from("jora"), Sub::from("radu"), Sub::from("igor")],
        });
        repo.create(&expected).await.unwrap();

        let actual = repo
            .find_by_id_and_sub(expected.id(), &Sub::from("jora"))
            .await
            .unwrap();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn should_not_find_by_id_and_sub() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::mongo::Config::test(&node).await.connect();
        let repo = MongoTalkRepository::new(&db);

        let talk_id = talk::Id::random();
        let actual = repo
            .find_by_id_and_sub(&talk_id, &Sub::from("valera"))
            .await
            .unwrap_err();

        assert!(matches!(actual, talk::Error::NotFound(Some(id)) if id.eq(&talk_id)));
    }

    #[tokio::test]
    async fn should_delete() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::mongo::Config::test(&node).await.connect();
        let repo = MongoTalkRepository::new(&db);

        let t = Talk::from(Details::Chat {
            members: [Sub::from("jora"), Sub::from("valera")],
        });
        repo.create(&t).await.unwrap();

        let deleted = repo.delete(t.id()).await.unwrap();

        assert!(deleted);
    }

    #[tokio::test]
    async fn should_not_delete() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::mongo::Config::test(&node).await.connect();
        let repo = MongoTalkRepository::new(&db);

        let talk_id = talk::Id::random();
        let deleted = repo.delete(&talk_id).await.unwrap();

        assert!(!deleted);
    }

    #[tokio::test]
    async fn should_return_true_when_talk_with_given_subs_exists() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::mongo::Config::test(&node).await.connect();
        let repo = MongoTalkRepository::new(&db);

        let t = Talk::from(Details::Chat {
            members: [Sub::from("jora"), Sub::from("valera")],
        });
        repo.create(&t).await.unwrap();

        let exists = repo
            .exists(&[Sub::from("valera"), Sub::from("jora")])
            .await
            .unwrap();

        assert!(exists);
    }

    #[tokio::test]
    async fn should_return_false_when_talk_with_given_subs_does_not_exist() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::mongo::Config::test(&node).await.connect();
        let repo = MongoTalkRepository::new(&db);

        let exists = repo
            .exists(&[Sub::from("valera"), Sub::from("jora")])
            .await
            .unwrap();

        assert!(!exists);
    }

    #[tokio::test]
    async fn should_update_last_message() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::mongo::Config::test(&node).await.connect();
        let repo = MongoTalkRepository::new(&db);

        let t = Talk::from(Details::Chat {
            members: [Sub::from("jora"), Sub::from("valera")],
        });
        repo.create(&t).await.unwrap();

        let pm = LastMessage::new(
            message::Id::random(),
            "hi!",
            Sub::from("jora"),
            chrono::Utc::now().timestamp(),
            true,
        );

        let lm = LastMessage::new(
            message::Id::random(),
            "bye!",
            Sub::from("valera"),
            chrono::Utc::now().timestamp(),
            false,
        );

        repo.update_last_message(t.id(), Some(&pm)).await.unwrap();
        repo.update_last_message(t.id(), Some(&lm)).await.unwrap();

        let res = repo.find_by_id(t.id()).await.unwrap();

        assert!(res.last_message().is_some_and(|r| lm.eq(&r)))
    }

    #[tokio::test]
    async fn should_set_last_message_to_none() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::mongo::Config::test(&node).await.connect();
        let repo = MongoTalkRepository::new(&db);

        let t = Talk::from(Details::Chat {
            members: [Sub::from("jora"), Sub::from("valera")],
        });
        repo.create(&t).await.unwrap();

        let lm = LastMessage::new(
            message::Id::random(),
            "bye!",
            Sub::from("valera"),
            chrono::Utc::now().timestamp(),
            false,
        );

        repo.update_last_message(t.id(), Some(&lm)).await.unwrap();
        repo.update_last_message(t.id(), None).await.unwrap();

        let res = repo.find_by_id(t.id()).await.unwrap();

        assert!(res.last_message().is_none())
    }

    #[tokio::test]
    async fn should_mark_as_seen() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::mongo::Config::test(&node).await.connect();
        let repo = MongoTalkRepository::new(&db);

        let t = Talk::from(Details::Chat {
            members: [Sub::from("jora"), Sub::from("valera")],
        });
        repo.create(&t).await.unwrap();

        let lm = LastMessage::new(
            message::Id::random(),
            "bye!",
            Sub::from("valera"),
            chrono::Utc::now().timestamp(),
            false,
        );

        repo.update_last_message(t.id(), Some(&lm)).await.unwrap();
        repo.mark_as_seen(t.id()).await.unwrap();

        let res = repo.find_by_id(t.id()).await.unwrap();

        assert!(res.last_message().is_some_and(|r| r.seen()))
    }

    #[tokio::test]
    async fn should_not_mark_as_seen_when_last_message_is_missing() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::mongo::Config::test(&node).await.connect();
        let repo = MongoTalkRepository::new(&db);

        let t = Talk::from(Details::Chat {
            members: [Sub::from("jora"), Sub::from("valera")],
        });
        repo.create(&t).await.unwrap();

        repo.update_last_message(t.id(), None).await.unwrap();
        repo.mark_as_seen(t.id()).await.unwrap();

        let res = repo.find_by_id(t.id()).await.unwrap();

        assert!(res.last_message().is_none())
    }
}
