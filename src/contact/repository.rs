use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::{Database, bson::doc};

use crate::user;

use super::{Id, Status, model::Contact};

const CONTACTS_COLLECTION: &str = "contacts";

#[async_trait]
pub trait ContactRepository {
    async fn find(&self, u1: &user::Id, u2: &user::Id) -> super::Result<Option<Contact>>;

    async fn find_by_id(&self, id: &Id) -> super::Result<Option<Contact>>;

    async fn find_by_user_id(&self, user_id: &user::Id) -> super::Result<Vec<Contact>>;

    async fn find_by_user_id_and_status(
        &self,
        user_id: &user::Id,
        s: &Status,
    ) -> super::Result<Vec<Contact>>;

    async fn add(&self, contact: &Contact) -> super::Result<()>;

    async fn update_status(&self, c: &Contact) -> super::Result<bool>;

    async fn delete(&self, me: &user::Id, you: &user::Id) -> super::Result<bool>;

    async fn exists(&self, u1: &user::Id, u2: &user::Id) -> super::Result<bool>;
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

#[async_trait]
impl ContactRepository for MongoContactRepository {
    async fn find(&self, u1: &user::Id, u2: &user::Id) -> super::Result<Option<Contact>> {
        let filter = doc! {};
        // let filter = doc! { "$or": [ {"sub1": sub1, "sub2": sub2}, {"sub2": sub1, "sub1": sub2} ] };

        self.col.find_one(filter).await.map_err(super::Error::from)
    }

    async fn find_by_id(&self, id: &Id) -> super::Result<Option<Contact>> {
        self.col
            .find_one(doc! { "_id": id })
            .await
            .map_err(super::Error::from)
    }

    async fn find_by_user_id(&self, user_id: &user::Id) -> super::Result<Vec<Contact>> {
        let filter = doc! {};
        // let filter = doc! { "$or": [ {"sub1": sub}, {"sub2": sub} ] };

        let cursor = self.col.find(filter).await?;

        cursor.try_collect().await.map_err(super::Error::from)
    }

    async fn find_by_user_id_and_status(
        &self,
        user_id: &user::Id,
        s: &Status,
    ) -> super::Result<Vec<Contact>> {
        let filter = doc! {
            "$or": [
                // { "sub1": sub, "status": s },
                // { "sub2": sub, "status": s }
            ]
        };

        let cursor = self.col.find(filter).await?;

        cursor.try_collect().await.map_err(super::Error::from)
    }

    async fn add(&self, c: &Contact) -> super::Result<()> {
        assert_ne!(c.user_id1(), c.user_id2());

        self.col.insert_one(c).await?;

        Ok(())
    }

    async fn update_status(&self, c: &Contact) -> super::Result<bool> {
        let res = self
            .col
            .update_one(
                doc! { "_id": c.id() },
                doc! { "$set": { "status": c.status() } },
            )
            .await?;

        Ok(res.modified_count > 0)
    }

    async fn delete(&self, me: &user::Id, you: &user::Id) -> super::Result<bool> {
        let filter = doc! { "$or": [ {"sub1": me, "sub2": you}, {"sub2": me, "sub1": you} ] };

        let res = self.col.delete_one(filter).await?;

        Ok(res.deleted_count > 0)
    }

    async fn exists(&self, u1: &user::Id, u2: &user::Id) -> super::Result<bool> {
        let filter = doc! {
            "$or": [
                // {"sub1": sub1, "sub2": sub2},
                // {"sub2": sub1, "sub1": sub2}
            ]
        };

        let result = self.col.find_one(filter).await?;
        Ok(result.is_some())
    }
}

// FIXME:
// #[cfg(test)]
// mod test {
//     use testcontainers_modules::{mongo::Mongo, testcontainers::runners::AsyncRunner};

//     use crate::{
//         contact::{self, model::Contact},
//         integration::db,
//         user::Sub,
//     };

//     use super::*;

//     #[tokio::test]
//     async fn should_find() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoContactRepository::new(&db);

//         let jora = Sub::from("jora");
//         let valera = Sub::from("valera");
//         let expected = Contact::new(&jora, &valera);
//         repo.add(&expected).await.unwrap();

//         let actual = repo.find(&jora, &valera).await.unwrap().unwrap();
//         assert_eq!(actual, expected);

//         let actual = repo.find(&valera, &jora).await.unwrap().unwrap();
//         assert_eq!(actual, expected);
//     }

//     #[tokio::test]
//     async fn should_not_find() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoContactRepository::new(&db);

//         let jora = Sub::from("jora");
//         let valera = Sub::from("valera");
//         let expected = Contact::new(&jora, &valera);
//         repo.add(&expected).await.unwrap();

//         let actual = repo.find(&jora, &Sub::from("radu")).await.unwrap();
//         assert!(actual.is_none());
//     }

//     #[tokio::test]
//     async fn should_find_by_id() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoContactRepository::new(&db);

//         let expected = Contact::new(&Sub::from("jora"), &Sub::from("valera"));
//         repo.add(&expected).await.unwrap();

//         let actual = repo.find_by_id(expected.id()).await.unwrap().unwrap();

//         assert_eq!(actual, expected);
//     }

//     #[tokio::test]
//     async fn should_not_find_by_id() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoContactRepository::new(&db);

//         let actual = repo.find_by_id(&contact::Id::random()).await.unwrap();

//         assert!(actual.is_none());
//     }

//     #[tokio::test]
//     async fn should_find_by_sub() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoContactRepository::new(&db);

//         let jora = Sub::from("jora");
//         let igor = Sub::from("igor");
//         let c1 = &Contact::new(&jora, &Sub::from("valera"));
//         let c2 = &Contact::new(&igor, &jora);
//         let c3 = &Contact::new(&igor, &Sub::from("radu"));

//         tokio::try_join!(repo.add(c1), repo.add(c2), repo.add(c3)).unwrap();

//         let mut expected = vec![c1, c2].into_iter();

//         let actual = repo.find_by_user_id(&jora).await.unwrap();

//         assert_eq!(actual.len(), expected.len());
//         assert!(expected.all(|c| actual.contains(c)));
//     }

//     #[tokio::test]
//     async fn should_not_find_by_sub() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoContactRepository::new(&db);

//         let actual = repo.find_by_user_id(&Sub::from("jora")).await.unwrap();

//         assert!(actual.is_empty());
//     }

//     #[tokio::test]
//     async fn should_find_by_sub_and_status() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoContactRepository::new(&db);

//         let jora = Sub::from("jora");
//         let valera = Sub::from("valera");
//         let igor = Sub::from("igor");

//         let mut c1 = Contact::new(&jora, &valera);
//         c1.set_status(Status::Rejected);
//         let mut c2 = Contact::new(&Sub::from("radu"), &jora);
//         c2.set_status(Status::Accepted);
//         let mut c3 = Contact::new(&jora, &igor);
//         c3.set_status(Status::Accepted);
//         let mut c4 = Contact::new(&igor, &valera);
//         c4.set_status(Status::Accepted);
//         let mut c5 = Contact::new(&Sub::from("ion"), &jora);
//         c5.set_status(Status::Blocked {
//             initiator: jora.clone(),
//         });

//         tokio::try_join!(
//             repo.add(&c1),
//             repo.add(&c2),
//             repo.add(&c3),
//             repo.add(&c4),
//             repo.add(&c5)
//         )
//         .unwrap();

//         let mut expected = vec![c2, c3].into_iter();

//         let actual = repo
//             .find_by_sub_and_status(&jora, &Status::Accepted)
//             .await
//             .unwrap();

//         assert_eq!(expected.len(), actual.len());
//         assert!(expected.all(|c| actual.contains(&c)))
//     }

//     #[tokio::test]
//     #[should_panic]
//     async fn should_panic_when_calling_add_with_same_subs() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoContactRepository::new(&db);

//         let jora = Sub::from("jora");
//         let c = Contact::new(&jora, &jora);

//         repo.add(&c).await.unwrap();
//     }

//     #[tokio::test]
//     async fn should_update_status() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoContactRepository::new(&db);

//         let initiator = Sub::from("jora");
//         let mut original = Contact::new(&initiator, &Sub::from("valera"));
//         original.set_status(Status::Pending { initiator });
//         repo.add(&original).await.unwrap();

//         let mut updated = original.clone();
//         updated.set_status(Status::Rejected);
//         let updated = repo.update_status(&updated).await.unwrap();
//         assert!(updated);

//         let res = repo.find_by_id(&original.id()).await.unwrap().unwrap();
//         assert_eq!(res.status(), &Status::Rejected);
//     }

//     #[tokio::test]
//     async fn should_not_update_status() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoContactRepository::new(&db);

//         let initiator = Sub::from("jora");
//         let mut not_persisted = Contact::new(&initiator, &Sub::from("valera"));
//         not_persisted.set_status(Status::Rejected);

//         let updated = repo.update_status(&not_persisted).await.unwrap();
//         assert!(!updated);
//     }

//     #[tokio::test]
//     async fn should_delete() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoContactRepository::new(&db);

//         let jora = Sub::from("jora");
//         let valera = Sub::from("valera");
//         let c = Contact::new(&jora, &valera);
//         repo.add(&c).await.unwrap();
//         assert!(repo.find_by_id(&c.id()).await.unwrap().is_some());

//         let deleted = repo.delete(&jora, &valera).await.unwrap();

//         assert!(deleted);
//         assert!(repo.find_by_id(&c.id()).await.unwrap().is_none());
//     }

//     #[tokio::test]
//     async fn should_delete_swapped() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoContactRepository::new(&db);

//         let jora = Sub::from("jora");
//         let valera = Sub::from("valera");
//         let c = Contact::new(&jora, &valera);
//         repo.add(&c).await.unwrap();
//         assert!(repo.find_by_id(&c.id()).await.unwrap().is_some());

//         let deleted = repo.delete(&valera, &jora).await.unwrap();

//         assert!(deleted);
//         assert!(repo.find_by_id(&c.id()).await.unwrap().is_none());
//     }

//     #[tokio::test]
//     async fn should_not_delete() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoContactRepository::new(&db);

//         let jora = Sub::from("jora");
//         let valera = Sub::from("valera");
//         let c = Contact::new(&jora, &valera);
//         repo.add(&c).await.unwrap();
//         assert!(repo.find_by_id(&c.id()).await.unwrap().is_some());

//         let deleted = repo.delete(&jora, &Sub::from("radu")).await.unwrap();

//         assert!(!deleted);
//         assert!(repo.find_by_id(&c.id()).await.unwrap().is_some());
//     }

//     #[tokio::test]
//     async fn should_return_true_when_exists_for_subs() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoContactRepository::new(&db);

//         let jora = Sub::from("jora");
//         let valera = Sub::from("valera");
//         let c = Contact::new(&jora, &valera);
//         repo.add(&c).await.unwrap();

//         let exists = repo.exists(&jora, &valera).await.unwrap();

//         assert!(exists);
//     }

//     #[tokio::test]
//     async fn should_return_true_when_exists_for_subs_swapped() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoContactRepository::new(&db);

//         let jora = Sub::from("jora");
//         let valera = Sub::from("valera");
//         let c = Contact::new(&jora, &valera);
//         repo.add(&c).await.unwrap();

//         let exists = repo.exists(&valera, &jora).await.unwrap();

//         assert!(exists);
//     }

//     #[tokio::test]
//     async fn should_return_false_when_does_not_exist_for_subs() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoContactRepository::new(&db);

//         let valera = Sub::from("valera");
//         let c = Contact::new(&Sub::from("jora"), &valera);
//         repo.add(&c).await.unwrap();

//         let exists = repo.exists(&valera, &Sub::from("radu")).await.unwrap();

//         assert!(!exists);
//     }
// }
