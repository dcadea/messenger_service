use axum::{
    routing::{get, post},
    Router,
};

use crate::{state::AppState, user};

type Result<T> = std::result::Result<T, Error>;
pub(crate) type Id = mongodb::bson::oid::ObjectId;

pub(crate) fn pages<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/", get(handler::home))
        .route("/chats", get(markup::all_chats))
        .route("/chats/:id", get(markup::active_chat))
        .with_state(state)
}

pub(crate) fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/chats", get(markup::all))
        .route("/chats/:id", get(markup::one))
        .route("/chats", post(handler::create))
        .with_state(state)
}

mod handler {
    use axum::{extract::State, Extension, Json};

    use crate::{markup::Wrappable, user::model::UserInfo};

    use super::{model::ChatRequest, service::ChatService};

    pub async fn home(logged_user: Extension<UserInfo>) -> Wrappable {
        Wrappable(super::markup::all_chats(logged_user).await)
    }

    pub async fn create(
        user_info: Extension<UserInfo>,
        chat_service: State<ChatService>,
        Json(chat_request): Json<ChatRequest>,
    ) -> crate::Result<()> {
        let _ = chat_service.create(&chat_request, &user_info).await?;

        Ok(())
    }
}

mod markup {
    use axum::extract::{Path, State};
    use axum::Extension;
    use maud::{html, Markup, Render};

    use crate::message::markup::message_input;
    use crate::user::markup::{UserHeader, UserSearch};
    use crate::user::model::UserInfo;
    use crate::user::service::UserService;

    use super::model::ChatDto;
    use super::service::ChatService;
    use super::Id;

    pub async fn all_chats(logged_user: Extension<UserInfo>) -> Markup {
        html! {
            #chat-window ."flex flex-col h-full" {
                (UserHeader{
                    name: &logged_user.name,
                    picture: &logged_user.picture,
                })

                (UserSearch{})

                #chat-list
                    hx-get="/api/chats"
                    hx-trigger="load"
                    hx-swap="outerHTML" {}
            }
        }
    }

    pub async fn active_chat(
        chat_id: Path<Id>,
        logged_user: Extension<UserInfo>,
        chat_service: State<ChatService>,
        user_service: State<UserService>,
    ) -> crate::Result<Markup> {
        let chat = chat_service.find_by_id(&chat_id, &logged_user).await?;
        let recipient = user_service.find_user_info(&chat.recipient).await?;

        Ok(html! {
            header class="flex justify-between items-center" {
                a class="border-2 border-red-500 text-red-500 px-4 py-2 rounded-2xl mr-4"
                    hx-get="/chats"
                    hx-target="#chat-window"
                    hx-swap="outerHTML" { "X" }
                h2 class="text-2xl" { (recipient.name) }
                img class="w-12 h-12 rounded-full"
                    src=(recipient.picture) alt="User avatar" {}
            }

            div ."flex-grow overflow-y-auto mt-4 mb-4"
                hx-get={ "/api/messages?limit=25&chat_id=" (chat.id) }
                hx-trigger="load" {}

            (message_input(&chat_id, &recipient.sub))
        })
    }

    pub async fn all(
        user_info: Extension<UserInfo>,
        chat_service: State<ChatService>,
    ) -> crate::Result<Markup> {
        let chats = chat_service.find_all(&user_info).await?;
        Ok(html! {
            #chat-list class="flex flex-col" {
                @for chat in chats {
                    (chat)
                }
            }
        })
    }

    pub async fn one(
        user_info: Extension<UserInfo>,
        chat_service: State<ChatService>,
        Path(id): Path<Id>,
    ) -> crate::Result<Markup> {
        let chat = chat_service.find_by_id(&id, &user_info).await?;
        Ok(chat.render())
    }

    impl Render for ChatDto {
        fn render(&self) -> Markup {
            html! {
                div class="chat-item p-4 mb-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex justify-between"
                    id={"c-" (self.id)}
                    hx-get={"/chats/" (self.id)}
                    hx-target="#chat-window" {

                    span."chat-recipient font-bold" { (self.recipient) }
                    @if let Some(last_message) = &self.last_message {
                        span class="chat-last-message text-sm text-gray-500 truncate" { (last_message) }
                    }
                }
            }
        }
    }
}

pub(crate) mod repository {
    use futures::stream::TryStreamExt;
    use mongodb::bson::doc;

    use crate::{chat, user};

    use super::model::Chat;
    use super::Id;

    const CHATS_COLLECTION: &str = "chats";

    pub struct ChatRepository {
        collection: mongodb::Collection<Chat>,
    }

    impl ChatRepository {
        pub fn new(database: &mongodb::Database) -> Self {
            Self {
                collection: database.collection(CHATS_COLLECTION),
            }
        }
    }

    impl ChatRepository {
        /**
         * Insert a new chat into the database
         * @param chat: The chat to insert
         */
        pub async fn insert(&self, chat: &Chat) -> super::Result<Chat> {
            let result = self.collection.insert_one(chat).await?;
            if let Some(id) = result.inserted_id.as_object_id() {
                return self.find_by_id(&id).await;
            }

            Err(chat::Error::Unexpected("Failed to insert chat".to_owned()))
        }

        /**
         * Update the last message of a chat
         * @param id: The id of the chat
         * @param text: The text of the last message
         * @param updated_at: The timestamp of the last message
         */
        pub async fn update_last_message(&self, id: &Id, text: &str) -> super::Result<()> {
            self.collection
                .update_one(
                    doc! { "_id": id },
                    doc! {"$set": {
                        "last_message": text,
                        "updated_at": chrono::Utc::now().timestamp(),
                    }},
                )
                .await?;
            Ok(())
        }

        pub async fn find_by_id(&self, id: &Id) -> super::Result<Chat> {
            self.collection
                .find_one(doc! { "_id": id })
                .await?
                .ok_or(chat::Error::NotFound(Some(*id)))
        }

        /**
         * Find a chat where the user sub is a member
         * @param sub: The user sub
         */
        pub async fn find_by_sub(&self, sub: &user::Sub) -> super::Result<Vec<Chat>> {
            let cursor = self
                .collection
                .find(doc! {"members": sub})
                .sort(doc! {"updated_at": -1})
                .await?;

            let chats = cursor.try_collect::<Vec<Chat>>().await?;

            Ok(chats)
        }

        /**
         * Find a chat by its id and the user sub
         * @param id: The id of the chat
         * @param sub: The user sub
         */
        pub async fn find_by_id_and_sub(&self, id: &Id, sub: &user::Sub) -> super::Result<Chat> {
            self.collection
                .find_one(doc! {
                    "_id": id,
                    "members": sub
                })
                .await?
                .ok_or(chat::Error::NotFound(Some(id.to_owned())))
        }

        /**
         * Find a chat id by its members
         * @param members: The members of the chat
         */
        pub async fn find_id_by_members(&self, members: [user::Sub; 2]) -> super::Result<Id> {
            let result = self
                .collection
                .find_one(doc! {
                    "members": { "$all": members.to_vec() }
                })
                .await?;

            if let Some(chat) = result {
                if let Some(id) = chat.id {
                    return Ok(id);
                }
            }

            Err(chat::Error::NotFound(None))
        }
    }
}

pub(crate) mod service {
    use std::sync::Arc;

    use futures::future::try_join_all;
    use futures::TryFutureExt;
    use redis::AsyncCommands;

    use super::model::{Chat, ChatDto, ChatRequest};
    use super::repository::ChatRepository;
    use super::Id;
    use crate::integration::cache;
    use crate::message::model::Message;
    use crate::user::model::UserInfo;
    use crate::user::service::UserService;
    use crate::{chat, user};

    const CHAT_TTL: i64 = 3600;

    #[derive(Clone)]
    pub struct ChatService {
        repository: Arc<ChatRepository>,
        user_service: Arc<UserService>,
        redis_con: redis::aio::ConnectionManager,
    }

    impl ChatService {
        pub fn new(
            repository: ChatRepository,
            user_service: UserService,
            redis_con: redis::aio::ConnectionManager,
        ) -> Self {
            Self {
                repository: Arc::new(repository),
                user_service: Arc::new(user_service),
                redis_con,
            }
        }
    }

    impl ChatService {
        pub async fn create(
            &self,
            req: &ChatRequest,
            user_info: &UserInfo,
        ) -> super::Result<ChatDto> {
            let owner = user_info.clone().sub;
            let recipient = req.clone().recipient;

            match self
                .repository
                .find_id_by_members([owner.to_owned(), recipient.to_owned()])
                .await
            {
                Ok(_) => Err(chat::Error::AlreadyExists([owner, recipient])),
                Err(chat::Error::NotFound(_)) => {
                    let chat = self
                        .repository
                        .insert(&Chat::new([owner.clone(), recipient.clone()]))
                        .await?;

                    self.user_service.add_friend(&owner, &recipient).await?;

                    self.chat_to_dto(chat, user_info).await
                }
                Err(err) => Err(err),
            }
        }

        pub async fn update_last_message(&self, message: &Message) -> super::Result<()> {
            let chat_id = self
                .repository
                .find_id_by_members([message.owner.to_owned(), message.recipient.to_owned()])
                .await?;

            self.repository
                .update_last_message(&chat_id, &message.text)
                .await
        }

        pub async fn find_by_id(&self, id: &Id, user_info: &UserInfo) -> super::Result<ChatDto> {
            match self.repository.find_by_id_and_sub(id, &user_info.sub).await {
                Ok(chat) => {
                    let chat_dto = self.chat_to_dto(chat, user_info).await?;
                    Ok(chat_dto)
                }
                Err(chat::Error::NotFound(_)) => Err(chat::Error::NotMember),
                Err(err) => Err(err),
            }
        }

        pub async fn find_all(&self, user_info: &UserInfo) -> super::Result<Vec<ChatDto>> {
            let chats = self.repository.find_by_sub(&user_info.sub).await?;

            let chat_dtos = try_join_all(
                chats
                    .into_iter()
                    .map(|chat| async { self.chat_to_dto(chat, user_info).await }),
            )
            .await?;

            Ok(chat_dtos)
        }
    }

    // validations
    impl ChatService {
        pub async fn check_member(&self, chat_id: &Id, sub: &user::Sub) -> super::Result<()> {
            let members = self.find_members(chat_id).await?;
            let belongs_to_chat = members.contains(sub);

            if !belongs_to_chat {
                return Err(chat::Error::NotMember);
            }

            Ok(())
        }

        pub async fn check_members(
            &self,
            chat_id: &Id,
            members: [user::Sub; 2],
        ) -> super::Result<()> {
            let cached_members = self.find_members(chat_id).await?;
            let belongs_to_chat =
                cached_members.contains(&members[0]) && cached_members.contains(&members[1]);

            if !belongs_to_chat {
                return Err(chat::Error::NotMember);
            }

            Ok(())
        }
    }

    // cache operations
    impl ChatService {
        pub async fn find_members(&self, chat_id: &Id) -> super::Result<[user::Sub; 2]> {
            let mut con = self.redis_con.clone();

            let cache_key = cache::Key::Chat(chat_id.to_owned());
            let members: Option<Vec<user::Sub>> = con.smembers(cache_key.clone()).await?;

            if members.as_ref().is_some_and(|m| m.len() == 2) {
                let members = members.unwrap();
                return Ok([members[0].clone(), members[1].clone()]);
            }

            let chat = self.repository.find_by_id(chat_id).await?;
            let members = chat.members;

            let _: () = con
                .clone()
                .sadd(&cache_key, &members.clone())
                .and_then(|_: ()| con.expire(&cache_key, CHAT_TTL))
                .await?;

            Ok(members)
        }
    }

    impl ChatService {
        async fn chat_to_dto(&self, chat: Chat, user_info: &UserInfo) -> super::Result<ChatDto> {
            let members = chat.members.to_owned();

            let recipient = members
                .iter()
                .find(|&m| m != &user_info.sub) // someone who is not a logged user :)
                .ok_or(chat::Error::NotMember)?;

            let recipient_info = self.user_service.find_user_info(recipient).await?;

            let chat_dto = ChatDto::new(chat, recipient.to_owned(), recipient_info.name);

            Ok(chat_dto)
        }
    }
}

mod model {
    use messenger_service::serde::serialize_object_id;
    use mongodb::bson::serde_helpers::serialize_object_id_as_hex_string;
    use serde;
    use serde::{Deserialize, Serialize};

    use crate::user;

    use super::Id;

    #[derive(Serialize, Deserialize)]
    pub struct Chat {
        #[serde(
            alias = "_id",
            serialize_with = "serialize_object_id",
            skip_serializing_if = "Option::is_none"
        )]
        pub id: Option<Id>,
        pub members: [user::Sub; 2],
        #[serde(skip_serializing_if = "Option::is_none")]
        pub last_message: Option<String>,
        updated_at: i64,
    }

    impl Chat {
        pub fn new(members: [user::Sub; 2]) -> Self {
            Self {
                id: None,
                members,
                last_message: None,
                updated_at: 0,
            }
        }
    }

    #[derive(Serialize)]
    pub struct ChatDto {
        #[serde(serialize_with = "serialize_object_id_as_hex_string")]
        pub id: Id,
        pub recipient: user::Sub,
        recipient_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub last_message: Option<String>,
        updated_at: i64,
    }

    impl ChatDto {
        pub fn new(chat: Chat, recipient: user::Sub, recipient_name: String) -> Self {
            let chat_id = chat.id.expect("No way chat id is missing!?");
            Self {
                id: chat_id,
                recipient,
                recipient_name,
                last_message: chat.last_message,
                updated_at: chat.updated_at,
            }
        }
    }

    #[derive(Deserialize, Clone)]
    pub struct ChatRequest {
        pub recipient: user::Sub,
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub(crate) enum Error {
    #[error("chat not found: {0:?}")]
    NotFound(Option<Id>),
    #[error("chat already exists for members: {0:?}")]
    AlreadyExists([user::Sub; 2]),
    #[error("user is not a member of the chat")]
    NotMember,
    #[error("unexpected chat error: {0}")]
    Unexpected(String),

    _User(#[from] user::Error),

    _MongoDB(#[from] mongodb::error::Error),
    _Redis(#[from] redis::RedisError),
}
