use maud::{html, Markup};

use crate::{chat, message, user};

use super::model::Notification;

pub fn noti_item(noti: &Notification, logged_sub: &user::Sub) -> Markup {
    match noti {
        Notification::NewMessage { msg } => {
            html! {
                div id="message-list"
                    hx-swap-oob="afterbegin"
                {
                    (message::markup::MessageItem::new(&msg, logged_sub))
                }
            }
        }
        Notification::UpdatedMessage { id: _, text: _ } => todo!(),
        Notification::DeletedMessage { id } => {
            html! {
                div id={"m-" (id.0)}
                    ."message-item flex items-center items-baseline" {
                    div ."message-bubble flex flex-row rounded-lg p-2 mt-2 max-w-xs"
                        ."bg-gray-300 text-gray-600 italic" {
                        "message deleted..."
                    }
                }
            }
        }
        Notification::SeenMessage { id } => {
            html! {
                div id={"m-" (id.0)} hx-swap-oob="beforeend" {
                    (message::markup::SeenIcon)
                }
            }
        }
        Notification::OnlineFriends { friends } => {
            html! {
                @for friend in friends {
                    (chat::markup::OnlineIcon { sub: friend, swappable: true })
                }
            }
        }
        Notification::NewFriend { chat_dto } => {
            html! {
                div id="chat-list" hx-swap-oob="afterbegin" {
                    (chat_dto)
                }
            }
        }
    }
}
