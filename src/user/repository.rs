use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::Database;
use mongodb::bson::doc;

use super::Nickname;
use super::Sub;
use super::model::User;

const USERS_COLLECTION: &str = "users";

#[async_trait]
pub trait UserRepository {
    async fn insert(&self, user: &User) -> super::Result<bool>;

    async fn find_by_sub(&self, sub: &Sub) -> super::Result<User>;

    async fn search_by_nickname_excluding(
        &self,
        nickname: &Nickname,
        exclude: &Nickname,
    ) -> super::Result<Vec<User>>;
}

pub struct MongoUserRepository {
    col: mongodb::Collection<User>,
}

impl MongoUserRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            col: db.collection(USERS_COLLECTION),
        }
    }
}

#[async_trait]
impl UserRepository for MongoUserRepository {
    async fn insert(&self, user: &User) -> super::Result<bool> {
        self.col.insert_one(user).await?;
        Ok(true)
    }

    async fn find_by_sub(&self, sub: &Sub) -> super::Result<User> {
        let filter = doc! { "sub": sub };
        let result = self.col.find_one(filter).await?;
        result.ok_or(super::Error::NotFound(sub.to_owned()))
    }

    // search users by nickname excluding the authenticated user
    async fn search_by_nickname_excluding(
        &self,
        nickname: &Nickname,
        exclude: &Nickname,
    ) -> super::Result<Vec<User>> {
        let filter = doc! {
            "$and": [
                { "nickname": { "$ne": exclude } },
                { "nickname": { "$regex": nickname, "$options": "i" } },
            ]
        };

        let cursor = self.col.find(filter).await?;

        cursor.try_collect().await.map_err(super::Error::from)
    }
}

#[cfg(test)]
mod test {
    use testcontainers_modules::{mongo::Mongo, testcontainers::runners::AsyncRunner};

    use super::{MongoUserRepository, UserRepository};

    use crate::{
        integration::db,
        user::{self, Email, Nickname, Picture, Sub, model::User},
    };

    #[tokio::test]
    async fn should_insert_user() {
        // TODO: switch to reusable containers (https://github.com/testcontainers/testcontainers-rs/issues/742)
        let node = Mongo::default().start().await.unwrap();
        let db = db::Config::test(&node).await.connect();
        let repo = MongoUserRepository::new(&db);

        let sub = Sub::from("test|123");
        let user = User::new(
            user::Id::random(),
            sub.clone(),
            Nickname::from("valera_kardan"),
            "valera".to_owned(),
            Picture::parse("picture").unwrap(),
            Email::parse("valera@test.com").unwrap(),
        );

        let inserted = repo.insert(&user).await.unwrap();
        assert!(inserted);

        let actual = repo.find_by_sub(&sub).await.unwrap();
        assert_eq!(actual, user);
    }

    #[tokio::test]
    async fn should_not_find_by_sub() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::Config::test(&node).await.connect();
        let repo = MongoUserRepository::new(&db);

        let sub = Sub::from("valera");

        let actual = repo.find_by_sub(&sub).await.unwrap_err();
        assert!(matches!(actual, user::Error::NotFound(s) if s.eq(&sub)));
    }

    #[tokio::test]
    async fn should_search_by_nickname_excluding() {
        let node = Mongo::default().start().await.unwrap();
        let db = db::Config::test(&node).await.connect();
        let repo = MongoUserRepository::new(&db);

        let valera = &User::new(
            user::Id::random(),
            Sub::from("test|123"),
            Nickname::from("valera_kardan"),
            "valera",
            Picture::parse("picture").unwrap(),
            Email::parse("valera@test.com").unwrap(),
        );

        let jora = &User::new(
            user::Id::random(),
            Sub::from("test|456"),
            Nickname::from("jora_partizan"),
            "jora",
            Picture::parse("picture").unwrap(),
            Email::parse("jora@test.com").unwrap(),
        );

        let radu = &User::new(
            user::Id::random(),
            Sub::from("test|135"),
            Nickname::from("radu_carlig"),
            "radu",
            Picture::parse("picture").unwrap(),
            Email::parse("radu@test.com").unwrap(),
        );

        let igor = &User::new(
            user::Id::random(),
            Sub::from("test|246"),
            Nickname::from("igor_frina"),
            "igor",
            Picture::parse("picture").unwrap(),
            Email::parse("igor@test.com").unwrap(),
        );

        tokio::try_join!(
            repo.insert(valera),
            repo.insert(jora),
            repo.insert(radu),
            repo.insert(igor)
        )
        .unwrap();

        let mut expected = vec![valera, jora].into_iter();

        let actual = repo
            .search_by_nickname_excluding(&Nickname::from("ra"), &Nickname::from("radu_carlig"))
            .await
            .unwrap();

        assert_eq!(expected.len(), actual.len());
        assert!(expected.all(|u| actual.contains(u)));
    }
}
