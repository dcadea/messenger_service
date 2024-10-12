use model::Sub;

type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    #[error("user not found: {:?}", 0)]
    NotFound(Sub),

    _MongoDB(#[from] mongodb::error::Error),
    _Redis(#[from] redis::RedisError),
    _ParseJson(#[from] serde_json::Error),
}

pub mod markup {
    use axum::extract::State;
    use axum::response::IntoResponse;
    use axum::routing::get;
    use axum::{Json, Router};
    use axum_extra::extract::Query;
    use maud::{html, Markup, Render};

    use crate::error::Error;
    use crate::state::AppState;

    use super::model::UserParams;
    use super::service::UserService;

    pub fn resources<S>(state: AppState) -> Router<S> {
        Router::new()
            .route("/users", get(find_handler))
            .with_state(state)
    }

    async fn find_handler(
        Query(params): Query<UserParams>,
        user_service: State<UserService>,
    ) -> impl IntoResponse {
        match params.sub {
            Some(sub) => match user_service.find_user_info(&sub).await {
                Ok(user_info) => Json(user_info).into_response(),
                Err(err) => Error::from(err).into_response(),
            },
            None => match params.nickname {
                Some(nickname) => match user_service.search_user_info(&nickname).await {
                    Ok(user_infos) => Json(user_infos).into_response(),
                    Err(err) => Error::from(err).into_response(),
                },
                None => Error::QueryParamRequired("sub or nickname".to_owned()).into_response(),
            },
        }
    }

    pub(crate) struct UserHeader<'a> {
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
}

pub mod model {
    use std::fmt::Display;

    use serde::{Deserialize, Serialize};

    type UserId = mongodb::bson::oid::ObjectId;

    #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    pub struct Sub(pub String); // TODO: remove pub

    impl Display for Sub {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl Serialize for Sub {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            self.0.serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for Sub {
        fn deserialize<D>(deserializer: D) -> Result<Sub, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            Ok(Sub(s))
        }
    }

    impl From<Sub> for mongodb::bson::Bson {
        fn from(val: Sub) -> Self {
            mongodb::bson::Bson::String(val.0)
        }
    }

    impl redis::FromRedisValue for Sub {
        fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Sub> {
            let s = String::from_redis_value(v)?;
            Ok(Sub(s))
        }
    }

    impl redis::ToRedisArgs for Sub {
        fn write_redis_args<W>(&self, out: &mut W)
        where
            W: ?Sized + redis::RedisWrite,
        {
            self.0.write_redis_args(out);
        }
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct User {
        #[serde(skip)]
        _id: Option<UserId>,
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
        nickname: String,
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

    impl redis::FromRedisValue for UserInfo {
        fn from_redis_value(value: &redis::Value) -> redis::RedisResult<Self> {
            let user_info: UserInfo = serde_json::from_str(&String::from_redis_value(value)?)?;
            Ok(user_info)
        }
    }

    impl redis::ToRedisArgs for UserInfo {
        fn write_redis_args<W>(&self, out: &mut W)
        where
            W: ?Sized + redis::RedisWrite,
        {
            serde_json::json!(self).to_string().write_redis_args(out);
        }
    }

    #[derive(Deserialize)]
    pub(super) struct UserParams {
        pub sub: Option<Sub>,
        pub nickname: Option<String>,
    }
}

pub mod repository {
    use futures::TryStreamExt;
    use mongodb::bson::doc;
    use mongodb::options::FindOneOptions;
    use mongodb::Database;

    use super::model::{Friends, Sub, User};
    use super::Result;
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
        pub async fn insert(&self, user: &User) -> Result<()> {
            self.users_col.insert_one(user).await?;
            Ok(())
        }

        pub async fn find_by_sub(&self, sub: &Sub) -> Result<User> {
            let filter = doc! { "sub": sub };
            let result = self.users_col.find_one(filter).await?;
            result.ok_or(user::Error::NotFound(sub.to_owned()))
        }

        pub async fn search_by_nickname(&self, nickname: &str) -> Result<Vec<User>> {
            let filter = doc! { "nickname":{
                "$regex": nickname,
                "$options": "i"
            }};

            let cursor = self.users_col.find(filter).await?;

            cursor.try_collect().await.map_err(user::Error::from)
        }

        pub async fn add_friend(&self, sub: &Sub, friend: &Sub) -> Result<()> {
            let filter = doc! { "sub": sub };
            let update = doc! { "$push": { "friends": friend } };

            self.users_col.update_one(filter, update).await?;
            Ok(())
        }

        pub async fn find_friends_by_sub(&self, sub: &Sub) -> Result<Vec<Sub>> {
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
                .ok_or(user::Error::NotFound(sub.to_owned()))
                .map(|f| f.friends)
        }
    }
}

pub mod service {
    use std::collections::HashSet;
    use std::sync::Arc;

    use redis::AsyncCommands;

    use crate::integration::model::CacheKey;
    use crate::user::model::{Sub, User, UserInfo};

    use super::repository::UserRepository;
    use super::Result;

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
        pub async fn create(&self, user: &User) -> Result<()> {
            self.repository.insert(user).await
        }

        pub async fn find_user_info(&self, sub: &Sub) -> Result<UserInfo> {
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

        pub async fn search_user_info(&self, nickname: &str) -> Result<Vec<UserInfo>> {
            let users = self.repository.search_by_nickname(nickname).await?;
            Ok(users.into_iter().map(|user| user.into()).collect())
        }

        pub async fn add_friend(&self, sub: &Sub, friend: &Sub) -> Result<()> {
            self.repository.add_friend(sub, friend).await?;
            self.cache_friend(sub, friend).await?;
            Ok(())
        }
    }

    // cache operations
    impl UserService {
        pub async fn add_online_user(&self, sub: &Sub) -> Result<()> {
            let mut con = self.redis_con.clone();
            let _: () = con.sadd(CacheKey::UsersOnline, sub).await?;
            Ok(())
        }

        pub async fn get_online_friends(&self, sub: &Sub) -> Result<HashSet<Sub>> {
            let mut con = self.redis_con.clone();
            let online_users: HashSet<Sub> = con
                .sinter(&[CacheKey::UsersOnline, CacheKey::Friends(sub.to_owned())])
                .await?;
            Ok(online_users)
        }

        pub async fn remove_online_user(&self, sub: &Sub) -> Result<()> {
            let mut con = self.redis_con.clone();
            let _: () = con.srem(CacheKey::UsersOnline, sub).await?;
            Ok(())
        }

        pub async fn cache_friends(&self, sub: &Sub) -> Result<()> {
            let friends = self.repository.find_friends_by_sub(sub).await?;

            let mut con = self.redis_con.clone();
            let _: () = con.sadd(CacheKey::Friends(sub.to_owned()), friends).await?;
            Ok(())
        }

        async fn cache_friend(&self, sub: &Sub, friend: &Sub) -> Result<()> {
            let mut con = self.redis_con.clone();
            let _: () = con.sadd(CacheKey::Friends(sub.to_owned()), friend).await?;
            Ok(())
        }

        async fn cache_user_info(&self, user_info: &UserInfo) -> Result<()> {
            let mut con = self.redis_con.clone();
            let cache_key = CacheKey::UserInfo(user_info.sub.to_owned());
            let _: () = con.set_ex(cache_key, user_info, USER_INFO_TTL).await?;
            Ok(())
        }

        async fn find_cached_user_info(&self, sub: &Sub) -> Option<UserInfo> {
            let mut con = self.redis_con.clone();
            let cache_key = CacheKey::UserInfo(sub.to_owned());
            let cached_user_info: Option<UserInfo> = con.get(cache_key).await.ok();
            cached_user_info
        }
    }
}
