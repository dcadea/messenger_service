use diesel::{
    Connection, ExpressionMethods, OptionalExtension, PgConnection, QueryDsl, QueryResult,
    RunQueryDsl, insert_into, r2d2::ConnectionManager, sql_query, sql_types,
};

use crate::{
    message::{self, model::LastMessage},
    schema::{chats, chats_users, groups, groups_users, talks},
    talk::{
        self, Kind,
        model::{
            ChatTalk, Details, GroupTalk, NewChat, NewChatUser, NewGroup, NewGroupUser, NewTalk,
            Talk,
        },
    },
    user,
};

pub trait TalkRepository {
    fn find_by_id(&self, id: &talk::Id) -> super::Result<Option<Talk>>;

    fn find_chats_by_user_id(&self, user_id: &user::Id) -> super::Result<Vec<ChatTalk>>;

    fn find_groups_by_user_id(&self, user_id: &user::Id) -> super::Result<Vec<GroupTalk>>;

    fn find_chat_by_id_and_user_id(
        &self,
        id: &talk::Id,
        user_id: &user::Id,
    ) -> super::Result<Option<ChatTalk>>;

    fn create(&self, t: &NewTalk) -> super::Result<talk::Id>;

    fn delete(&self, id: &talk::Id) -> super::Result<bool>;

    fn exists(&self, members: &[user::Id; 2]) -> super::Result<bool>;

    fn update_last_message(&self, id: &talk::Id, msg: Option<&LastMessage>) -> super::Result<()>;

    fn mark_as_seen(&self, id: &talk::Id) -> super::Result<()>;
}

#[derive(Clone)]
pub struct PgTalkRepository {
    pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}

impl PgTalkRepository {
    pub fn new(pool: r2d2::Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }
}

impl TalkRepository for PgTalkRepository {
    fn find_by_id(&self, t_id: &talk::Id) -> super::Result<Option<Talk>> {
        let mut conn = self.pool.get()?;

        talks::table
            .find(t_id.0)
            .first::<Talk>(&mut conn)
            .optional()
            .map_err(super::Error::from)
    }

    fn find_chats_by_user_id(&self, u_id: &user::Id) -> super::Result<Vec<ChatTalk>> {
        let mut conn = self.pool.get()?;

        sql_query(
            r#"
            SELECT
                t.id,
                t.last_message_id,
                u.id AS recipient,
                u.name,
                u.picture
            FROM talks t
            JOIN chats_users cu_self ON cu_self.chat_id = t.id AND cu_self.user_id = $1
            JOIN chats_users cu_other ON cu_other.chat_id = t.id AND cu_other.user_id != $1
            JOIN users u ON u.id = cu_other.user_id
            WHERE t.kind = 'chat'
            "#,
        )
        .bind::<sql_types::Uuid, _>(u_id.get())
        .load::<ChatTalk>(&mut conn)
        .map_err(super::Error::from)
    }

    fn find_groups_by_user_id(&self, u_id: &user::Id) -> super::Result<Vec<GroupTalk>> {
        let mut conn = self.pool.get()?;

        sql_query("SELECT * FROM groups_by_user_id($1)")
            .bind::<sql_types::Uuid, _>(u_id.get())
            .load::<GroupTalk>(&mut conn)
            .map_err(super::Error::from)
    }

    fn find_chat_by_id_and_user_id(
        &self,
        t_id: &talk::Id,
        u_id: &user::Id,
    ) -> super::Result<Option<ChatTalk>> {
        let mut conn = self.pool.get()?;

        let query = sql_query(
            r#"
            SELECT
                t.id,
                t.last_message_id,
                u.id AS recipient,
                u.name,
                u.picture
            FROM talks t
            JOIN chats_users cu_self ON cu_self.chat_id = t.id AND cu_self.user_id = $1
            JOIN chats_users cu_other ON cu_other.chat_id = t.id AND cu_other.user_id != $1
            JOIN users u ON u.id = cu_other.user_id
            WHERE t.id = $2
            AND t.kind = 'chat'
            "#,
        )
        .bind::<sql_types::Uuid, _>(u_id.get())
        .bind::<sql_types::Uuid, _>(t_id.get());

        query
            .get_result::<ChatTalk>(&mut conn)
            .optional()
            .map_err(super::Error::from)
    }

    fn create(&self, t: &NewTalk) -> super::Result<talk::Id> {
        let mut conn = self.pool.get()?;

        let tx_res: QueryResult<talk::Id> = conn.transaction(|conn| {
            let new_talk = (
                talks::kind.eq(Kind::Chat),
                talks::last_message_id.eq::<Option<message::Id>>(None),
            );
            let t_id: talk::Id = insert_into(talks::table)
                .values(new_talk)
                .returning(talks::id)
                .get_result(conn)?;

            match t.details() {
                Details::Chat { members } => {
                    let c_id: talk::Id = insert_into(chats::table)
                        .values(NewChat::new(&t_id))
                        .returning(chats::id)
                        .get_result(conn)?;

                    let users: Vec<NewChatUser> = members
                        .into_iter()
                        .map(|m| NewChatUser::new(&c_id, m))
                        .collect();

                    insert_into(chats_users::table)
                        .values(users)
                        .execute(conn)?;
                }
                Details::Group {
                    name,
                    owner,
                    members,
                } => {
                    let g_id: talk::Id = insert_into(groups::table)
                        .values(NewGroup::new(&t_id, owner, name))
                        .returning(groups::id)
                        .get_result(conn)?;

                    let users: Vec<NewGroupUser> = members
                        .into_iter()
                        .map(|m| NewGroupUser::new(&g_id, m))
                        .collect();

                    insert_into(groups_users::table)
                        .values(users)
                        .execute(conn)?;
                }
            }
            Ok(t_id)
        });

        tx_res.map_err(super::Error::from)
    }

    fn delete(&self, _id: &talk::Id) -> super::Result<bool> {
        // let res = self.col.delete_one(doc! {"_id": id}).await?;

        // Ok(res.deleted_count > 0)
        todo!("delete")
    }

    fn exists(&self, members: &[user::Id; 2]) -> super::Result<bool> {
        use crate::schema::chats_users::dsl::*;

        let mut conn = self.pool.get()?;

        chats_users
            .filter(user_id.eq_any(members))
            .select(chat_id)
            .group_by(chat_id)
            .having(diesel::dsl::count_distinct(user_id).eq(2))
            .first::<talk::Id>(&mut conn)
            .optional()
            .map(|r| r.is_some())
            .map_err(super::Error::from)
    }

    fn update_last_message(&self, _id: &talk::Id, _msg: Option<&LastMessage>) -> super::Result<()> {
        // self.col
        //     .update_one(
        //         doc! { "_id": id },
        //         doc! {"$set": {
        //             /*"last_message": msg,*/
        //         }},
        //     )
        //     .await?;
        // Ok(())
        todo!("update_last_message")
    }

    fn mark_as_seen(&self, _id: &talk::Id) -> super::Result<()> {
        // self.col
        //     .update_one(
        //         doc! {
        //             "$and": [
        //                 {"_id": id },
        //                 { "last_message.seen": { "$exists": true }}
        //             ]
        //         },
        //         doc! {"$set": {
        //             "last_message.seen": true,
        //         }},
        //     )
        //     .await?;
        // Ok(())
        todo!("update_last_message")
    }
}

// FIXME
// #[cfg(test)]
// mod test {
//     use testcontainers_modules::{mongo::Mongo, testcontainers::runners::AsyncRunner};

//     use crate::{
//         integration::db,
//         message::{self, model::LastMessage},
//         talk::{
//             self,
//             model::{Details, Talk},
//         },
//         user::Sub,
//     };

//     use super::{MongoTalkRepository, TalkRepository};

//     #[tokio::test]
//     async fn should_find_by_id() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoTalkRepository::new(&db);

//         let expected = Talk::from(Details::Chat {
//             members: [Sub::from("jora"), Sub::from("valera")],
//         });
//         repo.create(&expected).await.unwrap();

//         let actual = repo.find_by_id(expected.id()).await.unwrap();

//         assert_eq!(actual, expected);
//     }

//     #[tokio::test]
//     async fn should_not_find_by_id() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoTalkRepository::new(&db);

//         let talk_id = talk::Id::random();
//         let actual = repo.find_by_id(&talk_id).await.unwrap_err();

//         assert!(matches!(actual, talk::Error::NotFound(Some(id)) if id.eq(&talk_id)));
//     }

//     #[tokio::test]
//     async fn should_find_by_sub_and_kind() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoTalkRepository::new(&db);

//         let t1 = &Talk::from(Details::Chat {
//             members: [Sub::from("jora"), Sub::from("valera")],
//         });
//         let t2 = &Talk::from(Details::Chat {
//             members: [Sub::from("jora"), Sub::from("igor")],
//         });
//         let t3 = &Talk::from(Details::Chat {
//             members: [Sub::from("radu"), Sub::from("igor")],
//         });
//         let t4 = &Talk::from(Details::Group {
//             name: "g1".into(),
//             owner: Sub::from("radu"),
//             members: vec![Sub::from("jora"), Sub::from("radu"), Sub::from("igor")],
//         });

//         tokio::try_join!(
//             repo.create(t1),
//             repo.create(t2),
//             repo.create(t3),
//             repo.create(t4),
//         )
//         .unwrap();

//         let mut expected = vec![t1, t2].into_iter();

//         let actual = repo
//             .find_by_sub_and_kind(&Sub::from("jora"), &talk::Kind::Chat)
//             .await
//             .unwrap();

//         assert_eq!(expected.len(), actual.len());
//         assert!(expected.all(|t| actual.contains(t)));
//     }

//     #[tokio::test]
//     async fn should_not_find_by_sub_and_kind() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoTalkRepository::new(&db);

//         let t1 = &Talk::from(Details::Chat {
//             members: [Sub::from("jora"), Sub::from("valera")],
//         });
//         let t2 = &Talk::from(Details::Group {
//             name: "g1".into(),
//             owner: Sub::from("radu"),
//             members: vec![Sub::from("jora"), Sub::from("radu"), Sub::from("igor")],
//         });

//         tokio::try_join!(repo.create(t1), repo.create(t2),).unwrap();

//         let actual = repo
//             .find_by_sub_and_kind(&Sub::from("radu"), &talk::Kind::Chat)
//             .await
//             .unwrap();

//         assert!(actual.is_empty());
//     }

//     #[tokio::test]
//     async fn should_find_chat_by_id_and_sub1() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoTalkRepository::new(&db);

//         let expected = Talk::from(Details::Chat {
//             members: [Sub::from("jora"), Sub::from("valera")],
//         });
//         repo.create(&expected).await.unwrap();

//         let actual = repo
//             .find_by_id_and_sub(expected.id(), &Sub::from("jora"))
//             .await
//             .unwrap();

//         assert_eq!(actual, expected);
//     }

//     #[tokio::test]
//     async fn should_find_chat_by_id_and_sub2() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoTalkRepository::new(&db);

//         let expected = Talk::from(Details::Chat {
//             members: [Sub::from("jora"), Sub::from("valera")],
//         });
//         repo.create(&expected).await.unwrap();

//         let actual = repo
//             .find_by_id_and_sub(expected.id(), &Sub::from("valera"))
//             .await
//             .unwrap();

//         assert_eq!(actual, expected);
//     }

//     #[tokio::test]
//     async fn should_find_group_by_id_and_sub1() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoTalkRepository::new(&db);

//         let expected = Talk::from(Details::Group {
//             name: "g1".into(),
//             owner: Sub::from("radu"),
//             members: vec![Sub::from("jora"), Sub::from("radu"), Sub::from("igor")],
//         });
//         repo.create(&expected).await.unwrap();

//         let actual = repo
//             .find_by_id_and_sub(expected.id(), &Sub::from("jora"))
//             .await
//             .unwrap();

//         assert_eq!(actual, expected);
//     }

//     #[tokio::test]
//     async fn should_not_find_by_id_and_sub() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoTalkRepository::new(&db);

//         let talk_id = talk::Id::random();
//         let actual = repo
//             .find_by_id_and_sub(&talk_id, &Sub::from("valera"))
//             .await
//             .unwrap_err();

//         assert!(matches!(actual, talk::Error::NotFound(Some(id)) if id.eq(&talk_id)));
//     }

//     #[tokio::test]
//     async fn should_delete() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoTalkRepository::new(&db);

//         let t = Talk::from(Details::Chat {
//             members: [Sub::from("jora"), Sub::from("valera")],
//         });
//         repo.create(&t).await.unwrap();

//         let deleted = repo.delete(t.id()).await.unwrap();

//         assert!(deleted);
//     }

//     #[tokio::test]
//     async fn should_not_delete() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoTalkRepository::new(&db);

//         let talk_id = talk::Id::random();
//         let deleted = repo.delete(&talk_id).await.unwrap();

//         assert!(!deleted);
//     }

//     #[tokio::test]
//     async fn should_return_true_when_talk_with_given_subs_exists() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoTalkRepository::new(&db);

//         let t = Talk::from(Details::Chat {
//             members: [Sub::from("jora"), Sub::from("valera")],
//         });
//         repo.create(&t).await.unwrap();

//         let exists = repo
//             .exists(&[Sub::from("valera"), Sub::from("jora")])
//             .await
//             .unwrap();

//         assert!(exists);
//     }

//     #[tokio::test]
//     async fn should_return_false_when_talk_with_given_subs_does_not_exist() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoTalkRepository::new(&db);

//         let exists = repo
//             .exists(&[Sub::from("valera"), Sub::from("jora")])
//             .await
//             .unwrap();

//         assert!(!exists);
//     }

//     #[tokio::test]
//     async fn should_update_last_message() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoTalkRepository::new(&db);

//         let t = Talk::from(Details::Chat {
//             members: [Sub::from("jora"), Sub::from("valera")],
//         });
//         repo.create(&t).await.unwrap();

//         let pm = LastMessage::new(
//             message::Id::random(),
//             "hi!",
//             Sub::from("jora"),
//             chrono::Utc::now().timestamp(),
//             true,
//         );

//         let lm = LastMessage::new(
//             message::Id::random(),
//             "bye!",
//             Sub::from("valera"),
//             chrono::Utc::now().timestamp(),
//             false,
//         );

//         repo.update_last_message(t.id(), Some(&pm)).await.unwrap();
//         repo.update_last_message(t.id(), Some(&lm)).await.unwrap();

//         let res = repo.find_by_id(t.id()).await.unwrap();

//         assert!(res.last_message().is_some_and(|r| lm.eq(&r)))
//     }

//     #[tokio::test]
//     async fn should_set_last_message_to_none() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoTalkRepository::new(&db);

//         let t = Talk::from(Details::Chat {
//             members: [Sub::from("jora"), Sub::from("valera")],
//         });
//         repo.create(&t).await.unwrap();

//         let lm = LastMessage::new(
//             message::Id::random(),
//             "bye!",
//             Sub::from("valera"),
//             chrono::Utc::now().timestamp(),
//             false,
//         );

//         repo.update_last_message(t.id(), Some(&lm)).await.unwrap();
//         repo.update_last_message(t.id(), None).await.unwrap();

//         let res = repo.find_by_id(t.id()).await.unwrap();

//         assert!(res.last_message().is_none())
//     }

//     #[tokio::test]
//     async fn should_mark_as_seen() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoTalkRepository::new(&db);

//         let t = Talk::from(Details::Chat {
//             members: [Sub::from("jora"), Sub::from("valera")],
//         });
//         repo.create(&t).await.unwrap();

//         let lm = LastMessage::new(
//             message::Id::random(),
//             "bye!",
//             Sub::from("valera"),
//             chrono::Utc::now().timestamp(),
//             false,
//         );

//         repo.update_last_message(t.id(), Some(&lm)).await.unwrap();
//         repo.mark_as_seen(t.id()).await.unwrap();

//         let res = repo.find_by_id(t.id()).await.unwrap();

//         assert!(res.last_message().is_some_and(|r| r.seen()))
//     }

//     #[tokio::test]
//     async fn should_not_mark_as_seen_when_last_message_is_missing() {
//         let node = Mongo::default().start().await.unwrap();
//         let db = db::mongo::Config::test(&node).await.connect();
//         let repo = MongoTalkRepository::new(&db);

//         let t = Talk::from(Details::Chat {
//             members: [Sub::from("jora"), Sub::from("valera")],
//         });
//         repo.create(&t).await.unwrap();

//         repo.update_last_message(t.id(), None).await.unwrap();
//         repo.mark_as_seen(t.id()).await.unwrap();

//         let res = repo.find_by_id(t.id()).await.unwrap();

//         assert!(res.last_message().is_none())
//     }
// }
