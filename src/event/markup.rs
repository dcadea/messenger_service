use maud::{html, Markup};

use crate::{message::markup::message_item, user};

use super::model::Notification;

pub fn noti_item(noti: &Notification, logged_sub: &user::Sub) -> Markup {
    match noti {
        Notification::NewMessage { message } => {
            html! {
                div id="message-list"
                    hx-swap-oob="afterbegin"
                {
                    (message_item(&message, logged_sub))
                }
            }
        }
        Notification::UpdatedMessage { id: _, text: _ } => todo!(),
        Notification::DeletedMessage { id: _ } => todo!(),
        Notification::SeenMessage { id: _ } => todo!(),
        Notification::OnlineUsers { users: _ } => todo!(),
    }
}
