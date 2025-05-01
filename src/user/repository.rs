use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::Database;
use mongodb::bson::doc;
use user::Sub;

use super::model::User;
use crate::user;

const USERS_COLLECTION: &str = "users";

#[async_trait]
pub trait UserRepository {
    async fn insert(&self, user: &User) -> super::Result<bool>;

    async fn find_by_sub(&self, sub: &Sub) -> super::Result<User>;

    async fn search_by_nickname_excluding(
        &self,
        nickname: &str,
        exclude: &str,
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
        nickname: &str,
        exclude: &str,
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
        user::{self, model::User},
    };

    #[tokio::test]
    async fn should_insert_user() {
        // TODO: switch to reusable containers (https://github.com/testcontainers/testcontainers-rs/issues/742)
        let node = Mongo::default().start().await.unwrap();
        let db = db::Config::test(&node).await.connect();
        let repo = MongoUserRepository::new(&db);

        let sub = user::Sub("test|123".into());
        let user = User::new(
            user::Id::random(),
            sub.clone(),
            "valera_kardan".to_owned(),
            "valera".to_owned(),
            "picture".to_owned(),
            "valera@test.com".to_owned(),
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

        let sub = user::Sub("valera".into());

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
            user::Sub("test|123".into()),
            "valera_kardan",
            "valera",
            "picture",
            "valera@test.com",
        );

        let jora = &User::new(
            user::Id::random(),
            user::Sub("test|456".into()),
            "jora_partizan",
            "jora",
            "picture",
            "jora@test.com",
        );

        let radu = &User::new(
            user::Id::random(),
            user::Sub("test|135".into()),
            "radu_carlig",
            "radu",
            "picture",
            "radu@test.com",
        );

        let igor = &User::new(
            user::Id::random(),
            user::Sub("test|246".into()),
            "igor_frina",
            "igor",
            "picture",
            "igor@test.com",
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
            .search_by_nickname_excluding("ra", "radu_carlig")
            .await
            .unwrap();

        assert_eq!(expected.len(), actual.len());
        assert!(expected.all(|u| actual.contains(u)));
    }
}
