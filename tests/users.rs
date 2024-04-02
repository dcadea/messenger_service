#[cfg(test)]
mod tests {
    use messenger_api::user::model::User;
    use messenger_api::user::repository::UserRepository;

    const MONGO_URI: &str = "mongodb://root:example@localhost:27017";

    #[tokio::test]
    async fn test_fetch_user() {
        // FIXME: use test db
        let database = mongodb::Client::with_uri_str(MONGO_URI)
            .await
            .unwrap()
            .database("messenger");

        let username = "user1";
        let repository = UserRepository::new(database);

        let result = repository.find_one(username).await;

        match result {
            Ok(user) => {
                assert!(
                    user.is_some(),
                    "Expected to find a user with username {}",
                    username
                );
                if let Some(user) = user {
                    assert_eq!(user.username(), username);
                }
            }
            Err(err) => panic!("Failed to fetch user: {}", err),
        }
    }

    #[tokio::test]
    async fn test_insert_user() {
        let database = mongodb::Client::with_uri_str(MONGO_URI)
            .await
            .unwrap()
            .database("test");

        let username = "user2";
        let user = &User::new(username, "password2");
        let repository = UserRepository::new(database);

        let result = repository.insert(user).await;

        match result {
            Ok(_) => {
                let user = repository.find_one(username).await.unwrap();
                assert!(
                    user.is_some(),
                    "Expected to find a user with username {}",
                    username
                );
            }
            Err(err) => panic!("Failed to insert user: {}", err),
        }

        repository.delete(username).await.unwrap();
    }

    #[tokio::test]
    async fn test_update_user() {
        let database = mongodb::Client::with_uri_str(MONGO_URI)
            .await
            .unwrap()
            .database("test");

        let username = "user3";
        let user = &User::new(username, "password3");
        let repository = UserRepository::new(database);
        repository.insert(user).await.unwrap();

        let user = &User::new(username, "new_password3");

        let result = repository.update(user).await;

        match result {
            Ok(_) => {
                let user = repository.find_one(username).await.unwrap();
                assert!(
                    user.is_some(),
                    "Expected to find a user with id {}",
                    username
                );
                if let Some(user) = user {
                    assert_eq!(user.password(), "new_password3");
                }
            }
            Err(err) => panic!("Failed to update user: {}", err),
        }

        repository.delete(username).await.unwrap();
    }

    #[tokio::test]
    async fn test_delete_user() {
        let database = mongodb::Client::with_uri_str(MONGO_URI)
            .await
            .unwrap()
            .database("test");

        let username = "user4";
        let user = &User::new(username, "password4");
        let repository = UserRepository::new(database);
        repository.insert(user).await.unwrap();

        let result = repository.delete(username).await;

        match result {
            Ok(_) => {
                let user = repository.find_one(username).await.unwrap();
                assert!(
                    user.is_none(),
                    "Expected to not find a user with id {}",
                    username
                );
            }
            Err(err) => panic!("Failed to delete user: {}", err),
        }
    }
}
