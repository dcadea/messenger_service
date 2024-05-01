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
//         let username = "user1";
//         let repository = UserRepository::new(&database);
//
//         let result = repository.find_one(username).await;
//
//         match result {
//             Some(user) => assert_eq!(user.username, username),
//             None => panic!("Failed to fetch user: {}", username),
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
//     //         serde_json::from_str(r#"{ "username": "user2", "password": "password2" }"#).unwrap();
//     //     let repository = UserRepository::new(&database);
//
//     //     let result = repository.insert(&user).await;
//
//     //     let username = "user2";
//     //     match result {
//     //         Ok(_) => {
//     //             let user = repository.find_one(username).await;
//     //             assert!(
//     //                 user.is_some(),
//     //                 "Expected to find a user with username {}",
//     //                 username
//     //             );
//     //         }
//     //         Err(err) => panic!("Failed to insert user: {}", err),
//     //     }
//
//     //     repository.delete(username).await.unwrap();
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
//     //         serde_json::from_str(r#"{ "username": "user3", "password": "password3" }"#).unwrap();
//
//     //     let repository = UserRepository::new(&database);
//     //     repository.insert(&user).await.unwrap();
//
//     //     let user: User =
//     //         serde_json::from_str(r#"{ "username": "user3", "password": "new_password3" }"#)
//     //             .unwrap();
//
//     //     let result = repository.update(user).await;
//
//     //     let username = "user3";
//     //     match result {
//     //         Ok(_) => {
//     //             let user = repository.find_one(username).await;
//     //             assert!(
//     //                 user.is_some(),
//     //                 "Expected to find a user with id {}",
//     //                 username
//     //             );
//     //             if let Some(user) = user {
//     //                 assert_eq!(user.password(), "new_password3");
//     //             }
//     //         }
//     //         Err(err) => panic!("Failed to update user: {}", err),
//     //     }
//
//     //     repository.delete(username).await.unwrap();
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
//     //         serde_json::from_str(r#"{ "username": "user4", "password": "password4" }"#).unwrap();
//     //     let repository = UserRepository::new(&database);
//     //     repository.insert(&user).await.unwrap();
//
//     //     let username = "user4";
//     //     let result = repository.delete(username).await;
//
//     //     match result {
//     //         Ok(_) => {
//     //             let user = repository.find_one(username).await;
//     //             assert!(
//     //                 user.is_none(),
//     //                 "Expected to not find a user with id {}",
//     //                 username
//     //             );
//     //         }
//     //         Err(err) => panic!("Failed to delete user: {}", err),
//     //     }
//     // }
// }
