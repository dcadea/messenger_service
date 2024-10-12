use std::fmt::Display;

use axum::{routing::post, Router};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

type Result<T> = std::result::Result<T, Error>;
type Id = mongodb::bson::oid::ObjectId;

pub(crate) fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/users/search", post(self::handler::search))
        .with_state(state)
}

pub(self) mod handler {
    use axum::{extract::State, response::IntoResponse, Form};
    use serde::Deserialize;

    use super::service::UserService;

    #[derive(Deserialize)]
    pub struct FindParams {
        nickname: String,
    }

    pub async fn search(
        user_service: State<UserService>,
        params: Form<FindParams>,
    ) -> impl IntoResponse {
        let users = match user_service.search_user_info(&params.nickname).await {
            Ok(users) => users,
            Err(err) => return crate::error::Error::from(err).into_response(),
        };

        super::markup::search_result(&users).into_response()
    }
}

pub(crate) mod markup {
    use maud::{html, Markup, Render};

    use super::model::UserInfo;

    pub struct UserHeader<'a> {
        pub name: &'a str,
        pub picture: &'a str,
    }

    impl Render for UserHeader<'_> {
        fn render(&self) -> Markup {
            html! {
                header."flex justify-between items-center mb-4" {
                    img."w-12 h-12 rounded-full mr-2"
                        src=(self.picture)
                        alt="User avatar" {}
                    h2.text-2xl {(self.name)}
                    a."bg-red-500 hover:bg-red-700 text-white font-bold py-2 px-4 rounded"
                        href="/logout" { "Logout" }
                }
            }
        }
    }

    pub struct UserSearch;

    impl Render for UserSearch {
        fn render(&self) -> Markup {
            html! {

                input."mb-4 w-full px-3 py-2 border border-gray-300 rounded-md"
                    type="search"
                    name="nickname"
                    placeholder="Search users..."
                    hx-post="/api/users/search"
                    hx-trigger="input changed delay:500ms, search"
                    hx-target="#search-results" {}

                #search-results ."relative" {}
            }
        }
    }

    pub(super) fn search_result(users: &Vec<UserInfo>) -> Markup {
        let search_result_class =
            "absolute w-full bg-white border border-gray-300 rounded-md shadow-lg";
        html! {
            @if users.is_empty() {
                ul class=({search_result_class}) {
                    li."px-3 py-2" { "No users found" }
                }
            } @else {
                ul class=({search_result_class}) {
                    @for user in users {
                        li."px-3 py-2 hover:bg-gray-200 cursor-pointer flex items-center" {
                            img."w-6 h-6 rounded-full mr-3"
                                src=(user.picture)
                                alt="User avatar" {}
                            div {
                                strong {(user.name)} (user.nickname)
                            }
                        }
                    }
                }
            }
        }
    }
}

pub(crate) mod model {
    use serde::{Deserialize, Serialize};

    use super::{Id, Sub};

    #[derive(Serialize, Deserialize, Clone)]
    pub struct User {
        #[serde(skip)]
        _id: Option<Id>,
        sub: Sub,
        nickname: String,
        name: String,
        picture: String,
        email: String,
        friends: Vec<Sub>, // vec of sub
    }

    #[derive(Deserialize)]
    pub struct Friends {
        pub friends: Vec<Sub>, // vec of sub
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct UserInfo {
        pub sub: Sub,
        pub nickname: String,
        pub name: String,
        pub picture: String,
        email: String,
    }

    impl From<User> for UserInfo {
        fn from(user: User) -> Self {
            UserInfo {
                sub: user.sub,
                nickname: user.nickname,
                name: user.name,
                picture: user.picture,
                email: user.email,
            }
        }
    }

    impl From<UserInfo> for User {
        fn from(info: UserInfo) -> Self {
            Self {
                _id: None,
                sub: info.sub,
                nickname: info.nickname,
                name: info.name,
                picture: info.picture,
                email: info.email,
                friends: vec![],
            }
        }
    }
}

pub(crate) mod repository {
    use futures::TryStreamExt;
    use mongodb::bson::doc;
    use mongodb::options::FindOneOptions;
    use mongodb::Database;

    use super::model::{Friends, User};
    use super::Sub;
    use crate::user;

    const USERS_COLLECTION: &str = "users";

    pub struct UserRepository {
        users_col: mongodb::Collection<User>,
        friends_col: mongodb::Collection<Friends>,
    }

    impl UserRepository {
        pub fn new(database: &Database) -> Self {
            Self {
                users_col: database.collection(USERS_COLLECTION),
                friends_col: database.collection(USERS_COLLECTION),
            }
        }
    }

    impl UserRepository {
        pub async fn insert(&self, user: &User) -> super::Result<()> {
            self.users_col.insert_one(user).await?;
            Ok(())
        }

        pub async fn find_by_sub(&self, sub: &Sub) -> super::Result<User> {
            let filter = doc! { "sub": sub };
            let result = self.users_col.find_one(filter).await?;
            result.ok_or(super::Error::NotFound(sub.to_owned()))
        }

        pub async fn search_by_nickname(&self, nickname: &str) -> super::Result<Vec<User>> {
            let filter = doc! { "nickname":{
                "$regex": nickname,
                "$options": "i"
            }};

            let cursor = self.users_col.find(filter).await?;

            cursor.try_collect().await.map_err(super::Error::from)
        }

        pub async fn add_friend(&self, sub: &Sub, friend: &Sub) -> super::Result<()> {
            let filter = doc! { "sub": sub };
            let update = doc! { "$push": { "friends": friend } };

            self.users_col.update_one(filter, update).await?;
            Ok(())
        }

        pub async fn find_friends_by_sub(&self, sub: &user::Sub) -> super::Result<Vec<user::Sub>> {
            let filter = doc! { "sub": sub };
            let projection = FindOneOptions::builder()
                .projection(doc! { "friends": 1 })
                .build();

            let friends = self
                .friends_col
                .find_one(filter)
                .with_options(projection)
                .await?;

            friends
                .ok_or(super::Error::NotFound(sub.to_owned()))
                .map(|f| f.friends)
        }
    }
}

pub(crate) mod service {
    use std::collections::HashSet;
    use std::sync::Arc;

    use redis::AsyncCommands;

    use crate::integration::cache;
    use crate::user::model::{User, UserInfo};

    use super::repository::UserRepository;
    use super::Sub;

    const USER_INFO_TTL: u64 = 3600;

    #[derive(Clone)]
    pub struct UserService {
        redis_con: redis::aio::ConnectionManager,
        repository: Arc<UserRepository>,
    }

    impl UserService {
        pub fn new(redis_con: redis::aio::ConnectionManager, repository: UserRepository) -> Self {
            Self {
                redis_con,
                repository: Arc::new(repository),
            }
        }
    }

    impl UserService {
        pub async fn create(&self, user: &User) -> super::Result<()> {
            self.repository.insert(user).await
        }

        pub async fn find_user_info(&self, sub: &Sub) -> super::Result<UserInfo> {
            let cached_user_info = self.find_cached_user_info(sub).await;

            match cached_user_info {
                Some(user_info) => Ok(user_info),
                None => {
                    let user_info = self.repository.find_by_sub(sub).await?.into();
                    self.cache_user_info(&user_info).await?;
                    Ok(user_info)
                }
            }
        }

        pub async fn search_user_info(&self, nickname: &str) -> super::Result<Vec<UserInfo>> {
            let users = self.repository.search_by_nickname(nickname).await?;
            Ok(users.into_iter().map(|user| user.into()).collect())
        }

        pub async fn add_friend(&self, sub: &Sub, friend: &Sub) -> super::Result<()> {
            self.repository.add_friend(sub, friend).await?;
            self.cache_friend(sub, friend).await?;
            Ok(())
        }
    }

    // cache operations
    impl UserService {
        pub async fn add_online_user(&self, sub: &Sub) -> super::Result<()> {
            let mut con = self.redis_con.clone();
            let _: () = con.sadd(cache::Key::UsersOnline, sub).await?;
            Ok(())
        }

        pub async fn get_online_friends(&self, sub: &Sub) -> super::Result<HashSet<Sub>> {
            let mut con = self.redis_con.clone();
            let online_users: HashSet<Sub> = con
                .sinter(&[cache::Key::UsersOnline, cache::Key::Friends(sub.to_owned())])
                .await?;
            Ok(online_users)
        }

        pub async fn remove_online_user(&self, sub: &Sub) -> super::Result<()> {
            let mut con = self.redis_con.clone();
            let _: () = con.srem(cache::Key::UsersOnline, sub).await?;
            Ok(())
        }

        pub async fn cache_friends(&self, sub: &Sub) -> super::Result<()> {
            let friends = self.repository.find_friends_by_sub(sub).await?;

            let mut con = self.redis_con.clone();
            let _: () = con
                .sadd(cache::Key::Friends(sub.to_owned()), friends)
                .await?;
            Ok(())
        }

        async fn cache_friend(&self, sub: &Sub, friend: &Sub) -> super::Result<()> {
            let mut con = self.redis_con.clone();
            let _: () = con
                .sadd(cache::Key::Friends(sub.to_owned()), friend)
                .await?;
            Ok(())
        }

        async fn cache_user_info(&self, user_info: &UserInfo) -> super::Result<()> {
            let mut con = self.redis_con.clone();
            let cache_key = cache::Key::UserInfo(user_info.sub.to_owned());
            let _: () = con.set_ex(cache_key, user_info, USER_INFO_TTL).await?;
            Ok(())
        }

        async fn find_cached_user_info(&self, sub: &Sub) -> Option<UserInfo> {
            let mut con = self.redis_con.clone();
            let cache_key = cache::Key::UserInfo(sub.to_owned());
            let cached_user_info: Option<UserInfo> = con.get(cache_key).await.ok();
            cached_user_info
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) struct Sub(pub String); // TODO: remove pub

impl Display for Sub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for Sub {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Sub {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Sub, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Sub(s))
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub(crate) enum Error {
    #[error("user not found: {:?}", 0)]
    NotFound(Sub),

    _MongoDB(#[from] mongodb::error::Error),
    _Redis(#[from] redis::RedisError),
    _ParseJson(#[from] serde_json::Error),
}
