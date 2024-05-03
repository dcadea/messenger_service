// #[cfg(test)]
// mod tests {
//     use messenger_api::user::repository::UserRepository;
//
//     const MONGO_URI: &str = "mongodb://root:example@localhost:27017";
//
//     #[tokio::test]
//     async fn test_fetch_user() {
//         // FIXME: use test db
//         let database = mongodb::Client::with_uri_str(MONGO_URI)
//             .await
//             .unwrap()
//             .database("messenger");
//
//         let nickname = "user1";
//         let repository = UserRepository::new(&database);
//
//         let result = repository.find_one(nickname).await;
//
//         match result {
//             Some(user) => assert_eq!(user.nickname, nickname),
//             None => panic!("Failed to fetch user: {}", nickname),
//         }
//     }
//
//     // #[tokio::test]
//     // async fn test_insert_user() {
//     //     let database = mongodb::Client::with_uri_str(MONGO_URI)
//     //         .await
//     //         .unwrap()
//     //         .database("test");
//
//     //     let user: User =
//     //         serde_json::from_str(r#"{ "nickname": "user2", "password": "password2" }"#).unwrap();
//     //     let repository = UserRepository::new(&database);
//
//     //     let result = repository.insert(&user).await;
//
//     //     let nickname = "user2";
//     //     match result {
//     //         Ok(_) => {
//     //             let user = repository.find_one(nickname).await;
//     //             assert!(
//     //                 user.is_some(),
//     //                 "Expected to find a user with nickname {}",
//     //                 nickname
//     //             );
//     //         }
//     //         Err(err) => panic!("Failed to insert user: {}", err),
//     //     }
//
//     //     repository.delete(nickname).await.unwrap();
//     // }
//
//     // #[tokio::test]
//     // async fn test_update_user() {
//     //     let database = mongodb::Client::with_uri_str(MONGO_URI)
//     //         .await
//     //         .unwrap()
//     //         .database("test");
//
//     //     let user: User =
//     //         serde_json::from_str(r#"{ "nickname": "user3", "password": "password3" }"#).unwrap();
//
//     //     let repository = UserRepository::new(&database);
//     //     repository.insert(&user).await.unwrap();
//
//     //     let user: User =
//     //         serde_json::from_str(r#"{ "nickname": "user3", "password": "new_password3" }"#)
//     //             .unwrap();
//
//     //     let result = repository.update(user).await;
//
//     //     let nickname = "user3";
//     //     match result {
//     //         Ok(_) => {
//     //             let user = repository.find_one(nickname).await;
//     //             assert!(
//     //                 user.is_some(),
//     //                 "Expected to find a user with id {}",
//     //                 nickname
//     //             );
//     //             if let Some(user) = user {
//     //                 assert_eq!(user.password(), "new_password3");
//     //             }
//     //         }
//     //         Err(err) => panic!("Failed to update user: {}", err),
//     //     }
//
//     //     repository.delete(nickname).await.unwrap();
//     // }
//
//     // #[tokio::test]
//     // async fn test_delete_user() {
//     //     let database = mongodb::Client::with_uri_str(MONGO_URI)
//     //         .await
//     //         .unwrap()
//     //         .database("test");
//
//     //     let user: User =
//     //         serde_json::from_str(r#"{ "nickname": "user4", "password": "password4" }"#).unwrap();
//     //     let repository = UserRepository::new(&database);
//     //     repository.insert(&user).await.unwrap();
//
//     //     let nickname = "user4";
//     //     let result = repository.delete(nickname).await;
//
//     //     match result {
//     //         Ok(_) => {
//     //             let user = repository.find_one(nickname).await;
//     //             assert!(
//     //                 user.is_none(),
//     //                 "Expected to not find a user with id {}",
//     //                 nickname
//     //             );
//     //         }
//     //         Err(err) => panic!("Failed to delete user: {}", err),
//     //     }
//     // }
// }
