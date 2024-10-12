use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Extension, Router};
use maud::{html, Markup, Render};

use crate::message::markup::message_input;
use crate::result::Result;
use crate::state::AppState;
use crate::user::markup::UserHeader;
use crate::user::model::UserInfo;
use crate::user::service::UserService;

use super::model::{ChatDto, ChatId};
use super::service::ChatService;

pub fn pages<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/chats", get(all_chats))
        .route("/chats/:id", get(active_chat))
        .with_state(state)
}

pub async fn all_chats(logged_user: Extension<UserInfo>) -> Result<Markup> {
    Ok(html! {
        #chat-window ."flex flex-col h-full"
            hx-get="/api/chats"
            hx-trigger="load"
            hx-swap="beforeend"
        {
            (UserHeader{
                name: &logged_user.name,
                picture: &logged_user.picture,
            })
        }
    })
}

async fn active_chat(
    chat_id: Path<ChatId>,
    logged_user: Extension<UserInfo>,
    chat_service: State<ChatService>,
    user_service: State<UserService>,
) -> Result<Markup> {
    let chat = chat_service.find_by_id(&chat_id, &logged_user).await?;
    let recipient = user_service.find_user_info(&chat.recipient).await?;

    Ok(html! {
        header class="flex justify-between items-center" {
            a class="border-2 border-red-500 text-red-500 px-4 py-2 rounded-2xl mr-4"
                hx-get="/chats"
                hx-target="#chat-window"
                hx-swap="innerHTML" { "X" }
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

pub fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/chats", get(find_all))
        .route("/chats/:id", get(find_one))
        .with_state(state)
}

async fn find_all(
    user_info: Extension<UserInfo>,
    chat_service: State<ChatService>,
) -> Result<Markup> {
    let chats = chat_service.find_all(&user_info).await?;
    Ok(html! {
        div class="chat-list flex flex-col" {
            @for chat in chats {
                (chat)
            }
        }
    })
}

async fn find_one(
    user_info: Extension<UserInfo>,
    chat_service: State<ChatService>,
    Path(id): Path<ChatId>,
) -> Result<Markup> {
    let chat = chat_service.find_by_id(&id, &user_info).await?;
    Ok(chat.render())
}

impl Render for ChatDto {
    fn render(&self) -> Markup {
        html! {
            div class="chat-item p-4 mb-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex justify-between"
                id={"c-" (self.id)}
                hx-get={"/chats/" (self.id)}
                hx-target="#chat-window"
                hx-swap="innerHTML" {

                span."chat-recipient font-bold" { (self.recipient) }
                @if let Some(last_message) = &self.last_message {
                    span class="chat-last-message text-sm text-gray-500 truncate" { (last_message) }
                }
            }
        }
    }
}

// async fn create_handler(
//     user_info: Extension<UserInfo>,
//     chat_service: State<ChatService>,
//     app_endpoints: State<AppEndpoints>,
//     Json(chat_request): Json<ChatRequest>,
// ) -> Result<impl IntoResponse> {
//     let base_url = app_endpoints.api();
//     let result = chat_service.create(&chat_request, &user_info).await?;
//     let location = format!("{base_url}/chats/{}", &result.id);

//     let mut response = Json(result).into_response();
//     *response.status_mut() = StatusCode::CREATED;
//     response
//         .headers_mut()
//         .insert(header::LOCATION, HeaderValue::from_str(&location)?);

//     Ok(response)
// }
