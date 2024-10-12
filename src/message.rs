use axum::{
    routing::{delete, get},
    Router,
};

use crate::state::AppState;

type Result<T> = std::result::Result<T, Error>;
pub(crate) type Id = mongodb::bson::oid::ObjectId;

pub(crate) fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/messages", get(markup::find_all))
        .route("/messages/:id", get(markup::find_one))
        .route("/messages/:id", delete(markup::delete_one))
        .with_state(state)
}

pub(crate) mod markup {
    use axum::extract::{Path, State};
    use axum::Extension;
    use axum_extra::extract::Query;
    use maud::{html, Markup};
    use serde::Deserialize;

    use crate::chat::service::ChatService;
    use crate::error::Error;
    use crate::user::model::UserInfo;
    use crate::{chat, user};

    use super::model::MessageDto;
    use super::service::MessageService;
    use super::Id;

    #[derive(Deserialize)]
    pub(super) struct Params {
        chat_id: Option<chat::Id>,
        end_time: Option<i64>,
        limit: Option<usize>,
    }

    pub(super) async fn find_all(
        user_info: Extension<UserInfo>,
        params: Query<Params>,
        chat_service: State<ChatService>,
        message_service: State<MessageService>,
    ) -> crate::Result<Markup> {
        let chat_id = params
            .chat_id
            .ok_or(Error::QueryParamRequired("chat_id".to_owned()))?;

        chat_service.check_member(&chat_id, &user_info.sub).await?;

        let messages = message_service
            .find_by_chat_id_and_params(&chat_id, params.limit, params.end_time)
            .await?;

        Ok(html! {
            div class="message-list flex flex-col" {
                @for msg in messages {
                    (message_item(&msg, &user_info))
                }
            }
        })
    }

    pub(super) async fn find_one(
        id: Path<Id>,
        user_info: Extension<UserInfo>,
        message_service: State<MessageService>,
    ) -> crate::Result<Markup> {
        // TODO: chat_service.check_member(&chat_id, &user_info.sub).await?;

        let msg = message_service.find_by_id(&id).await?;

        Ok(message_item(&msg, &user_info))
    }

    pub(super) async fn delete_one(
        id: Path<Id>,
        message_service: State<MessageService>,
    ) -> crate::Result<()> {
        // TODO: chat_service.check_member(&chat_id, &user_info.sub).await?;

        message_service.delete(&id).await?;

        Ok(())
    }

    pub fn message_input(chat_id: &chat::Id, recipient: &user::Sub) -> Markup {
        html! {
            form #message-input ."border-gray-200 flex"
            {
                input type="hidden" name="type" value="create_message" {}
                input type="hidden" name="chat_id" value=(chat_id) {}
                input type="hidden" name="recipient" value=(recipient) {}

                input ."border border-gray-300 rounded-l-md p-2 flex-1"
                    type="text"
                    name="text"
                    placeholder="Type your message..." {}

                input ."bg-blue-600 text-white px-4 rounded-r-md"
                    type="submit"
                    value="Send" {}
            }
        }
    }

    fn message_item(msg: &MessageDto, user_info: &UserInfo) -> Markup {
        let belongs_to_user = msg.owner == user_info.sub;
        let message_timestamp =
            chrono::DateTime::from_timestamp(msg.timestamp, 0).map(|dt| dt.format("%H:%M"));

        html! {
            .message-item
                id={"m-" (msg.id)}
                ."flex items-center items-baseline"
                .justify-end[belongs_to_user]
            {
                @if belongs_to_user {
                    i ."fa-trash-can fa-solid text-red-700 cursor-pointer"
                        hx-delete={"/api/messages/" (msg.id)}
                        hx-target={"#m-" (msg.id)}
                        hx-swap="outerHTML" {}

                    // TODO: Add edit handler
                    i ."fa-pen fa-solid ml-2 text-green-700 cursor-pointer" {}
                }

                div.message-bubble
                    ."flex flex-row rounded-lg p-2 mt-2 max-w-xs relative"
                    ."bg-blue-600 text-white ml-2"[belongs_to_user]
                    ."bg-gray-300 text-gray-600"[!belongs_to_user] {

                    p.message-text ."mr-3 whitespace-normal font-light" { (msg.text) }
                    @if let Some(mt) = message_timestamp {
                        span.message-timestamp .text-xs { (mt) }
                    }

                    @if belongs_to_user {
                        i ."fa-solid fa-check absolute bottom-1 right-1 opacity-65" {}

                        @if msg.seen {
                            i ."fa-solid fa-check absolute bottom-1 right-2.5 opacity-65" {}
                        }
                    }
                }
            }
        }
    }
}

pub(crate) mod model {
    use mongodb::bson::serde_helpers::serialize_object_id_as_hex_string;
    use serde::{Deserialize, Serialize};

    use crate::{chat, user};
    use messenger_service::serde::serialize_object_id;

    use super::Id;

    #[derive(Deserialize, Serialize, Clone)]
    pub struct Message {
        #[serde(
            alias = "_id",
            serialize_with = "serialize_object_id",
            skip_serializing_if = "Option::is_none"
        )]
        id: Option<Id>,
        chat_id: chat::Id,
        pub owner: user::Sub,
        pub recipient: user::Sub,
        pub text: String,
        timestamp: i64,
        seen: bool,
    }

    impl Message {
        pub fn new(chat_id: chat::Id, owner: user::Sub, recipient: user::Sub, text: &str) -> Self {
            Self {
                id: None,
                chat_id,
                owner,
                recipient,
                text: text.to_string(),
                timestamp: chrono::Utc::now().timestamp(),
                seen: false,
            }
        }

        pub fn with_id(&self, id: Id) -> Self {
            Self {
                id: Some(id),
                ..self.clone()
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct MessageDto {
        #[serde(serialize_with = "serialize_object_id_as_hex_string")]
        pub id: Id,
        #[serde(serialize_with = "serialize_object_id_as_hex_string")]
        chat_id: chat::Id,
        pub owner: user::Sub,
        pub recipient: user::Sub,
        pub text: String,
        pub timestamp: i64,
        pub seen: bool,
    }

    impl From<Message> for MessageDto {
        fn from(message: Message) -> Self {
            Self {
                id: message.id.expect("where is message id!?"),
                chat_id: message.chat_id,
                owner: message.owner.clone(),
                recipient: message.recipient.clone(),
                text: message.clone().text,
                timestamp: message.timestamp,
                seen: message.seen,
            }
        }
    }
}

pub(crate) mod repository {
    use futures::TryStreamExt;
    use mongodb::bson::doc;
    use mongodb::Database;

    use super::{model::Message, Id};
    use crate::{chat, message};

    const MESSAGES_COLLECTION: &str = "messages";

    pub struct MessageRepository {
        collection: mongodb::Collection<Message>,
    }

    impl MessageRepository {
        pub fn new(database: &Database) -> Self {
            Self {
                collection: database.collection(MESSAGES_COLLECTION),
            }
        }
    }

    impl MessageRepository {
        pub async fn insert(&self, message: &Message) -> super::Result<Id> {
            let result = self.collection.insert_one(message).await?;
            if let Some(id) = result.inserted_id.as_object_id() {
                return Ok(id.to_owned());
            }

            Err(message::Error::Unexpected(
                "Failed to insert message".to_owned(),
            ))
        }

        pub async fn find_by_id(&self, id: &Id) -> super::Result<Message> {
            self.collection
                .find_one(doc! {"_id": id})
                .await?
                .ok_or(message::Error::NotFound(Some(id.to_owned())))
        }

        pub async fn find_by_chat_id(&self, chat_id: &chat::Id) -> super::Result<Vec<Message>> {
            let cursor = self
                .collection
                .find(doc! {"chat_id": chat_id})
                .sort(doc! {"timestamp": 1})
                .await?;

            let messages = cursor.try_collect::<Vec<Message>>().await?;

            Ok(messages)
        }

        pub async fn find_by_chat_id_limited(
            &self,
            chat_id: &chat::Id,
            limit: usize,
        ) -> super::Result<Vec<Message>> {
            let cursor = self
                .collection
                .find(doc! {"chat_id": chat_id})
                .sort(doc! {"timestamp": -1})
                .limit(limit as i64)
                .await?;

            let messages = cursor
                .try_collect::<Vec<Message>>()
                .await
                .map(|mut messages| {
                    messages.reverse();
                    messages
                })?;

            Ok(messages)
        }

        pub async fn find_by_chat_id_before(
            &self,
            chat_id: &chat::Id,
            before: i64,
        ) -> super::Result<Vec<Message>> {
            let cursor = self
                .collection
                .find(doc! {
                    "chat_id": chat_id,
                    "timestamp": {"$lt": before}
                })
                .sort(doc! {"timestamp": 1})
                .await?;

            let messages = cursor.try_collect::<Vec<Message>>().await?;

            Ok(messages)
        }

        pub async fn find_by_chat_id_limited_before(
            &self,
            chat_id: &chat::Id,
            limit: usize,
            before: i64,
        ) -> super::Result<Vec<Message>> {
            let cursor = self
                .collection
                .find(doc! {
                    "chat_id": chat_id,
                    "timestamp": {"$lt": before}
                })
                .sort(doc! {"timestamp": -1})
                .limit(limit as i64)
                .await?;

            let messages = cursor
                .try_collect::<Vec<Message>>()
                .await
                .map(|mut messages| {
                    messages.reverse();
                    messages
                })?;

            Ok(messages)
        }

        pub async fn update(&self, id: &Id, text: &str) -> super::Result<()> {
            self.collection
                .update_one(doc! {"_id": id}, doc! {"$set": {"text": text}})
                .await?;
            Ok(())
        }

        pub async fn delete(&self, id: &Id) -> super::Result<()> {
            self.collection.delete_one(doc! {"_id": id}).await?;
            Ok(())
        }

        pub async fn mark_as_seen(&self, id: &Id) -> super::Result<()> {
            self.collection
                .update_one(doc! {"_id": id}, doc! {"$set": {"seen": true}})
                .await?;
            Ok(())
        }
    }
}

pub(crate) mod service {
    use std::sync::Arc;

    use crate::chat;

    use super::model::{Message, MessageDto};
    use super::repository::MessageRepository;
    use super::Id;

    #[derive(Clone)]
    pub struct MessageService {
        repository: Arc<MessageRepository>,
    }

    impl MessageService {
        pub fn new(repository: MessageRepository) -> Self {
            Self {
                repository: Arc::new(repository),
            }
        }
    }

    impl MessageService {
        pub async fn create(&self, message: &Message) -> super::Result<Message> {
            self.repository
                .insert(message)
                .await
                .map(|id| message.with_id(id))
        }

        pub async fn update(&self, id: &Id, text: &str) -> super::Result<()> {
            self.repository.update(id, text).await
        }

        pub async fn delete(&self, id: &Id) -> super::Result<()> {
            self.repository.delete(id).await
        }

        pub async fn mark_as_seen(&self, id: &Id) -> super::Result<()> {
            self.repository.mark_as_seen(id).await
        }
    }

    impl MessageService {
        pub async fn find_by_id(&self, id: &Id) -> super::Result<MessageDto> {
            self.repository
                .find_by_id(id)
                .await
                .map(|msg| MessageDto::from(msg))
        }

        pub async fn find_by_chat_id_and_params(
            &self,
            chat_id: &chat::Id,
            limit: Option<usize>,
            end_time: Option<i64>,
        ) -> super::Result<Vec<MessageDto>> {
            let result = match (limit, end_time) {
                (None, None) => self.repository.find_by_chat_id(chat_id).await?,
                (Some(limit), None) => {
                    self.repository
                        .find_by_chat_id_limited(chat_id, limit)
                        .await?
                }
                (None, Some(end_time)) => {
                    self.repository
                        .find_by_chat_id_before(chat_id, end_time)
                        .await?
                }
                (Some(limit), Some(end_time)) => {
                    self.repository
                        .find_by_chat_id_limited_before(chat_id, limit, end_time)
                        .await?
                }
            };

            let result = result
                .iter()
                .map(|msg| MessageDto::from(msg.clone()))
                .collect::<Vec<_>>();

            Ok(result)
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub(crate) enum Error {
    #[error("message not found: {0:?}")]
    NotFound(Option<Id>),
    #[error("unexpected message error: {0}")]
    Unexpected(String),

    _MongoDB(#[from] mongodb::error::Error),
}
