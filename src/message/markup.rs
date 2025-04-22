use chrono::DateTime;
use maud::{Markup, Render, html};

use crate::{markup::IdExt, message, talk, user};

use super::model::{LastMessage, Message};

const MESSAGE_INPUT_ID: &str = "message-input";
pub const MESSAGE_INPUT_TARGET: &str = "#message-input";

pub const MESSAGE_LIST_ID: &str = "message-list";
pub const MESSAGE_LIST_TARGET: &str = "#message-list";

pub struct InputBlank<'a>(pub &'a talk::Id);

impl Render for InputBlank<'_> {
    fn render(&self) -> Markup {
        let send_message_handler = format!(
            r"
                on htmx:afterRequest reset() me
                on htmx:afterRequest go to the bottom of the {MESSAGE_LIST_TARGET}
            "
        );

        html! {
            form #(MESSAGE_INPUT_ID) ."border-gray-200 flex mb-3"
                hx-post="/api/messages"
                hx-target=(MESSAGE_LIST_TARGET)
                hx-swap="afterbegin"
                _=(send_message_handler)
            {
                input type="hidden" name="talk_id" value=(self.0) {}
                (InputText(None))
                (SendButton)
            }
        }
    }
}

pub struct InputEdit<'a> {
    id: &'a message::Id,
    old_text: &'a str,
}

impl<'a> InputEdit<'a> {
    pub fn new(id: &'a message::Id, old_text: &'a str) -> Self {
        Self { id, old_text }
    }
}

impl Render for InputEdit<'_> {
    fn render(&self) -> Markup {
        html! {
            form #(MESSAGE_INPUT_ID) ."border-gray-200 flex mb-3"
                hx-put="/api/messages"
                hx-target=(self.id.target())
                hx-swap="outerHTML"
            {
                input type="hidden" name="message_id" value=(self.id) {}
                (InputText(Some(self.old_text)))
                (SendButton)
            }

        }
    }
}

struct InputText<'a>(Option<&'a str>);

impl Render for InputText<'_> {
    fn render(&self) -> Markup {
        html! {
            input ."border border-gray-300 rounded-l-md p-2 flex-1 focus:outline-none"
                type="text"
                name="text"
                value=[self.0]
                placeholder="Type your message..."
                autocomplete="off"
                hx-disabled-elt="this"
                _="on keyup if the event's key is 'Escape' set value of me to ''" {}
        }
    }
}

struct SendButton;

impl Render for SendButton {
    fn render(&self) -> Markup {
        html! {
            input ."bg-blue-600 text-white px-4 rounded-r-md cursor-pointer hover:bg-blue-700"
                hx-disabled-elt="this"
                type="submit"
                value="Send" {}
        }
    }
}

pub struct MessageItem<'a> {
    msg: &'a Message,
    auth_sub: Option<&'a user::Sub>,
    is_last: bool,
}

impl<'a> MessageItem<'a> {
    pub fn new(msg: &'a Message, auth_sub: Option<&'a user::Sub>) -> Self {
        Self {
            msg,
            auth_sub,
            is_last: false,
        }
    }

    fn as_last(&'a mut self) -> &'a Self {
        self.is_last = true;
        self
    }

    fn belongs_to_user(&self) -> bool {
        if let Some(sub) = self.auth_sub {
            self.msg.owner == *sub
        } else {
            false
        }
    }

    fn hx_trigger(&self) -> Option<&'a str> {
        if self.is_last {
            Some("intersect once")
        } else {
            None
        }
    }

    fn hx_swap(&self) -> Option<&'a str> {
        if self.is_last { Some("afterend") } else { None }
    }

    fn next_page(&self) -> Option<String> {
        if self.is_last {
            let path = format!(
                "/api/messages?limit=20&talk_id={}&end_time={}",
                self.msg.talk_id, self.msg.timestamp
            );
            Some(path)
        } else {
            None
        }
    }

    fn controls_handler(&self) -> Option<&str> {
        if self.belongs_to_user() {
            Some(
                r"
                on mouseover remove .hidden from the first <div.message-controls/> in me
                on mouseout add .hidden to the first <div.message-controls/> in me
                ",
            )
        } else {
            None
        }
    }
}

const MESSAGE_CLASS: &str = "message-item flex items-end relative";
const MESSAGE_BUBBLE_CLASS: &str = "message-bubble flex flex-row rounded-lg p-2 mt-2 max-w-xs";
const MESSAGE_TEXT_CLASS: &str =
    "message-text break-words overflow-hidden mr-2 whitespace-normal font-light";

impl Render for MessageItem<'_> {
    fn render(&self) -> Markup {
        let belongs_to_user = self.belongs_to_user();

        let msg_timestamp =
            DateTime::from_timestamp(self.msg.timestamp, 0).map(|dt| dt.format("%H:%M"));

        html! {
            div #(self.msg.id.attr())
                .(MESSAGE_CLASS)
                .justify-end[belongs_to_user]
                hx-trigger=[self.hx_trigger()]
                hx-swap=[self.hx_swap()]
                hx-get=[self.next_page()]
                _=[self.controls_handler()]
            {
                @if belongs_to_user {
                    div ."message-controls hidden pb-2" {
                        (Icon::Delete(&self.msg.id))
                        (Icon::Edit(&self.msg))
                    }

                    (Icon::Sent)

                    @if self.msg.seen {
                        (Icon::Seen)
                    }
                }

                div .(MESSAGE_BUBBLE_CLASS)
                    ."bg-blue-600 text-white ml-2"[belongs_to_user]
                    ."bg-gray-300 text-gray-600"[!belongs_to_user] {

                    p .(MESSAGE_TEXT_CLASS) lang="en" { (self.msg.text) }
                    @if let Some(timestamp) = msg_timestamp {
                        span ."message-timestamp text-xs opacity-65" { (timestamp) }
                    }
                }
            }
        }
    }
}

pub struct MessageList<'a> {
    messages: &'a [Message],
    sub: &'a user::Sub,
    append: bool,
}

impl<'a> MessageList<'a> {
    pub fn prepend(messages: &'a [Message], sub: &'a user::Sub) -> Self {
        Self {
            messages,
            sub,
            append: false,
        }
    }

    pub fn append(messages: &'a [Message], sub: &'a user::Sub) -> Self {
        Self {
            messages,
            sub,
            append: true,
        }
    }
}

impl Render for MessageList<'_> {
    fn render(&self) -> Markup {
        let sub = Some(self.sub);
        html! {
            @for i in 0..self.messages.len() {
                @if self.append && i == self.messages.len() - 1 {
                    (MessageItem::new(&self.messages[i], sub).as_last())
                } @else {
                    (MessageItem::new(&self.messages[i], sub))
                }
            }
        }
    }
}

const MAX_LEN: usize = 25;

pub fn last_message(
    lm: Option<&LastMessage>,
    talk_id: &talk::Id,
    sender: Option<&user::Sub>,
) -> Markup {
    let trim = |lm: &LastMessage| {
        let mut text = lm.text.clone();
        if text.len() > MAX_LEN {
            text.truncate(MAX_LEN);
            text.push_str("...");
        }
        text
    };

    html! {
        div #{"lm-"(talk_id)} ."last-message text-sm text-gray-500" {
            @if let Some(last_msg) = lm {
                (trim(last_msg))

                @if let Some(s) = sender {
                    @if !last_msg.seen && last_msg.owner != *s {
                        (talk::markup::Icon::Unseen)
                    }
                } @else {
                    (talk::markup::Icon::Unseen)
                }
            }
        }
    }
}

impl crate::markup::IdExt for message::Id {
    fn attr(&self) -> String {
        format!("m-{}", self.0)
    }

    fn target(&self) -> String {
        format!("#m-{}", self.0)
    }
}

pub enum Icon<'a> {
    Edit(&'a Message),
    Delete(&'a message::Id),
    Sent,
    Seen,
}

impl Render for Icon<'_> {
    fn render(&self) -> Markup {
        html! {
            @match self {
                Self::Edit(msg) =>{
                    i ."fa-pen fa-solid ml-2 text-green-700 cursor-pointer"
                        hx-get={"/templates/messages/input/edit?message_id=" (msg.id)}
                        hx-target=(MESSAGE_INPUT_TARGET)
                        hx-swap="outerHTML" {}
                },
                Self::Delete(id) => {
                    i ."fa-trash-can fa-solid text-red-700 cursor-pointer"
                        hx-delete={"/api/messages/" (id)}
                        hx-target=(id.target())
                        hx-swap="outerHTML swap:200ms" {}
                },
                Self::Sent => i ."fa-solid fa-check absolute bottom-1 right-1 text-white opacity-65" {},
                Self::Seen => i ."fa-solid fa-check absolute bottom-1 right-2.5 text-white opacity-65" {},
            }
        }
    }
}

#[cfg(test)]
mod test {
    use chrono::DateTime;
    use maud::Render;

    use crate::{
        markup::IdExt,
        message::{
            self,
            markup::MessageItem,
            model::{LastMessage, Message},
        },
        talk, user,
    };

    use super::{Icon, InputBlank, InputEdit, InputText, MessageList, SendButton, last_message};

    #[test]
    fn should_render_input_blank() {
        let talk_id = &talk::Id("67dff625c469e51787ba173d".to_string());

        let expected = concat!(
            "<form class=\"border-gray-200 flex mb-3\" id=\"message-input\" hx-post=\"/api/messages\" hx-target=\"#message-list\" hx-swap=\"afterbegin\" ",
            r#"_="
                on htmx:afterRequest reset() me
                on htmx:afterRequest go to the bottom of the #message-list
            ">"#,
            r#"<input type="hidden" name="talk_id" value="67dff625c469e51787ba173d"></input>"#,
            r#"<input class="border border-gray-300 rounded-l-md p-2 flex-1 focus:outline-none" type="text" name="text" placeholder="Type your message..." autocomplete="off" hx-disabled-elt="this" _="on keyup if the event's key is 'Escape' set value of me to ''"></input>"#,
            r#"<input class="bg-blue-600 text-white px-4 rounded-r-md cursor-pointer hover:bg-blue-700" hx-disabled-elt="this" type="submit" value="Send"></input>"#,
            "</form>",
        );

        let actual = InputBlank(talk_id).render();

        assert_eq!(expected, actual.into_string())
    }

    #[test]
    fn should_render_input_edit() {
        let message_id = &message::Id("67dff625c469e51787ba173d".to_string());

        let expected = concat!(
            "<form class=\"border-gray-200 flex mb-3\" id=\"message-input\" hx-put=\"/api/messages\" hx-target=\"#m-67dff625c469e51787ba173d\" hx-swap=\"outerHTML\">",
            r#"<input type="hidden" name="message_id" value="67dff625c469e51787ba173d"></input>"#,
            r#"<input class="border border-gray-300 rounded-l-md p-2 flex-1 focus:outline-none" type="text" name="text" value="old text" placeholder="Type your message..." autocomplete="off" hx-disabled-elt="this" _="on keyup if the event's key is 'Escape' set value of me to ''"></input>"#,
            r#"<input class="bg-blue-600 text-white px-4 rounded-r-md cursor-pointer hover:bg-blue-700" hx-disabled-elt="this" type="submit" value="Send"></input>"#,
            "</form>",
        );

        let actual = InputEdit::new(message_id, "old text").render();

        assert_eq!(expected, actual.into_string())
    }

    #[test]
    fn should_render_input_text_with_blank_value() {
        let expected = r#"<input class="border border-gray-300 rounded-l-md p-2 flex-1 focus:outline-none" type="text" name="text" placeholder="Type your message..." autocomplete="off" hx-disabled-elt="this" _="on keyup if the event's key is 'Escape' set value of me to ''"></input>"#;

        let actual = InputText(None).render();

        assert_eq!(expected, actual.into_string())
    }

    #[test]
    fn should_render_input_text_with_some_value() {
        let expected = r#"<input class="border border-gray-300 rounded-l-md p-2 flex-1 focus:outline-none" type="text" name="text" value="hello" placeholder="Type your message..." autocomplete="off" hx-disabled-elt="this" _="on keyup if the event's key is 'Escape' set value of me to ''"></input>"#;

        let actual = InputText(Some("hello")).render();

        assert_eq!(expected, actual.into_string())
    }

    #[test]
    fn should_render_send_button() {
        let expected = r#"<input class="bg-blue-600 text-white px-4 rounded-r-md cursor-pointer hover:bg-blue-700" hx-disabled-elt="this" type="submit" value="Send"></input>"#;

        let actual = SendButton.render();

        assert_eq!(expected, actual.into_string())
    }

    #[test]
    fn should_render_message_item_where_auth_user_is_owner() {
        let talk_id = talk::Id("67dff625c469e51787ba173d".to_string());
        let auth_sub = "google|jora";
        let msg = Message::new(talk_id, user::Sub(auth_sub.into()), "Lorem ipsum");

        let msg_id = &msg.id;
        let msg_timestamp = DateTime::from_timestamp(msg.timestamp, 0)
            .map(|dt| dt.format("%H:%M"))
            .unwrap();

        let expected = format!(
            concat!(
                r#"<div class="message-item flex items-end relative justify-end" id="m-{}" "#,
                r#"_="
                on mouseover remove .hidden from the first &lt;div.message-controls/&gt; in me
                on mouseout add .hidden to the first &lt;div.message-controls/&gt; in me
                ">"#,
                r#"<div class="message-controls hidden pb-2">"#,
                "<i class=\"fa-trash-can fa-solid text-red-700 cursor-pointer\" hx-delete=\"/api/messages/{}\" hx-target=\"#m-{}\" hx-swap=\"outerHTML swap:200ms\"></i>",
                "<i class=\"fa-pen fa-solid ml-2 text-green-700 cursor-pointer\" hx-get=\"/templates/messages/input/edit?message_id={}\" hx-target=\"#message-input\" hx-swap=\"outerHTML\"></i>",
                "</div>",
                r#"<i class="fa-solid fa-check absolute bottom-1 right-1 text-white opacity-65"></i>"#,
                r#"<div class="message-bubble flex flex-row rounded-lg p-2 mt-2 max-w-xs bg-blue-600 text-white ml-2">"#,
                r#"<p class="message-text break-words overflow-hidden mr-2 whitespace-normal font-light" lang="en">Lorem ipsum</p>"#,
                r#"<span class="message-timestamp text-xs opacity-65">{}</span>"#,
                "</div>",
                "</div>"
            ),
            msg_id, msg_id, msg_id, msg_id, msg_timestamp
        );

        let actual = MessageItem::new(&msg, Some(&user::Sub(auth_sub.into()))).render();

        assert_eq!(expected, actual.into_string())
    }

    #[test]
    fn should_render_message_item_where_auth_user_is_not_owner() {
        let talk_id = talk::Id("67dff625c469e51787ba173d".to_string());
        let msg = Message::new(talk_id, user::Sub("auth0|valera".into()), "Lorem ipsum");

        let msg_id = &msg.id;
        let msg_timestamp = DateTime::from_timestamp(msg.timestamp, 0)
            .map(|dt| dt.format("%H:%M"))
            .unwrap();

        let expected = format!(
            concat!(
                r#"<div class="message-item flex items-end relative" id="m-{}">"#,
                r#"<div class="message-bubble flex flex-row rounded-lg p-2 mt-2 max-w-xs bg-gray-300 text-gray-600">"#,
                r#"<p class="message-text break-words overflow-hidden mr-2 whitespace-normal font-light" lang="en">Lorem ipsum</p>"#,
                r#"<span class="message-timestamp text-xs opacity-65">{}</span>"#,
                "</div>",
                "</div>"
            ),
            msg_id, msg_timestamp
        );

        let actual = MessageItem::new(&msg, None).render();

        assert_eq!(expected, actual.into_string())
    }

    #[test]
    fn should_render_message_list_as_prepend() {
        let talk_id = talk::Id("67dff625c469e51787ba173d".to_string());
        let auth_sub = user::Sub("google|jora".into());

        let msg1 = Message::new(talk_id.clone(), auth_sub.clone(), "Lorem ipsum");
        let msg1_id = &msg1.id;
        let msg1_timestamp = DateTime::from_timestamp(msg1.timestamp, 0)
            .map(|dt| dt.format("%H:%M"))
            .unwrap();

        let msg2 = Message::new(
            talk_id,
            user::Sub("auth0|valera".into()),
            "Sed ut perspiciatis",
        );
        let msg2_id = &msg2.id;
        let msg2_timestamp = DateTime::from_timestamp(msg2.timestamp, 0)
            .map(|dt| dt.format("%H:%M"))
            .unwrap();

        let expected = format!(
            concat!(
                r#"<div class="message-item flex items-end relative justify-end" id="m-{}" "#,
                r#"_="
                on mouseover remove .hidden from the first &lt;div.message-controls/&gt; in me
                on mouseout add .hidden to the first &lt;div.message-controls/&gt; in me
                ">"#,
                r#"<div class="message-controls hidden pb-2">"#,
                "<i class=\"fa-trash-can fa-solid text-red-700 cursor-pointer\" hx-delete=\"/api/messages/{}\" hx-target=\"#m-{}\" hx-swap=\"outerHTML swap:200ms\"></i>",
                "<i class=\"fa-pen fa-solid ml-2 text-green-700 cursor-pointer\" hx-get=\"/templates/messages/input/edit?message_id={}\" hx-target=\"#message-input\" hx-swap=\"outerHTML\"></i>",
                "</div>",
                r#"<i class="fa-solid fa-check absolute bottom-1 right-1 text-white opacity-65"></i>"#,
                r#"<div class="message-bubble flex flex-row rounded-lg p-2 mt-2 max-w-xs bg-blue-600 text-white ml-2">"#,
                r#"<p class="message-text break-words overflow-hidden mr-2 whitespace-normal font-light" lang="en">Lorem ipsum</p>"#,
                r#"<span class="message-timestamp text-xs opacity-65">{}</span>"#,
                "</div>",
                "</div>",
                r#"<div class="message-item flex items-end relative" id="m-{}">"#,
                r#"<div class="message-bubble flex flex-row rounded-lg p-2 mt-2 max-w-xs bg-gray-300 text-gray-600">"#,
                r#"<p class="message-text break-words overflow-hidden mr-2 whitespace-normal font-light" lang="en">Sed ut perspiciatis</p>"#,
                r#"<span class="message-timestamp text-xs opacity-65">{}</span>"#,
                "</div>",
                "</div>"
            ),
            msg1_id, msg1_id, msg1_id, msg1_id, msg1_timestamp, msg2_id, msg2_timestamp,
        );

        let actual = MessageList::prepend(&[msg1, msg2], &auth_sub).render();

        assert_eq!(expected, actual.into_string())
    }

    #[test]
    fn should_render_message_list_as_append() {
        let talk_id = talk::Id("67dff625c469e51787ba173d".to_string());
        let auth_sub = user::Sub("google|jora".into());

        let msg1 = Message::new(talk_id.clone(), auth_sub.clone(), "Lorem ipsum");
        let msg1_id = &msg1.id;
        let msg1_timestamp = DateTime::from_timestamp(msg1.timestamp, 0)
            .map(|dt| dt.format("%H:%M"))
            .unwrap();

        let msg2 = Message::new(
            talk_id.clone(),
            user::Sub("auth0|valera".into()),
            "Sed ut perspiciatis",
        );
        let msg2_id = &msg2.id;
        let msg2_timestamp = DateTime::from_timestamp(msg2.timestamp, 0)
            .map(|dt| dt.format("%H:%M"))
            .unwrap();

        let expected = format!(
            concat!(
                r#"<div class="message-item flex items-end relative justify-end" id="m-{}" "#,
                r#"_="
                on mouseover remove .hidden from the first &lt;div.message-controls/&gt; in me
                on mouseout add .hidden to the first &lt;div.message-controls/&gt; in me
                ">"#,
                r#"<div class="message-controls hidden pb-2">"#,
                "<i class=\"fa-trash-can fa-solid text-red-700 cursor-pointer\" hx-delete=\"/api/messages/{}\" hx-target=\"#m-{}\" hx-swap=\"outerHTML swap:200ms\"></i>",
                "<i class=\"fa-pen fa-solid ml-2 text-green-700 cursor-pointer\" hx-get=\"/templates/messages/input/edit?message_id={}\" hx-target=\"#message-input\" hx-swap=\"outerHTML\"></i>",
                "</div>",
                r#"<i class="fa-solid fa-check absolute bottom-1 right-1 text-white opacity-65"></i>"#,
                r#"<div class="message-bubble flex flex-row rounded-lg p-2 mt-2 max-w-xs bg-blue-600 text-white ml-2">"#,
                r#"<p class="message-text break-words overflow-hidden mr-2 whitespace-normal font-light" lang="en">Lorem ipsum</p>"#,
                r#"<span class="message-timestamp text-xs opacity-65">{}</span>"#,
                "</div>",
                "</div>",
                r#"<div class="message-item flex items-end relative" id="m-{}" hx-trigger="intersect once" hx-swap="afterend" hx-get="/api/messages?limit=20&amp;talk_id={}&amp;end_time={}">"#,
                r#"<div class="message-bubble flex flex-row rounded-lg p-2 mt-2 max-w-xs bg-gray-300 text-gray-600">"#,
                r#"<p class="message-text break-words overflow-hidden mr-2 whitespace-normal font-light" lang="en">Sed ut perspiciatis</p>"#,
                r#"<span class="message-timestamp text-xs opacity-65">{}</span>"#,
                "</div>",
                "</div>"
            ),
            msg1_id,
            msg1_id,
            msg1_id,
            msg1_id,
            msg1_timestamp,
            msg2_id,
            &talk_id,
            msg2.timestamp,
            msg2_timestamp,
        );

        let actual = MessageList::append(&[msg1, msg2], &auth_sub).render();

        assert_eq!(expected, actual.into_string())
    }

    #[test]
    fn should_render_empty_last_message() {
        let talk_id = talk::Id("67dff625c469e51787ba173d".to_string());

        let expected = r#"<div class="last-message text-sm text-gray-500" id="lm-67dff625c469e51787ba173d"></div>"#;

        let actual = last_message(None, &talk_id, None);

        assert_eq!(expected, actual.into_string())
    }

    #[test]
    fn should_render_unassigned_last_message() {
        let talk_id = talk::Id("67dff625c469e51787ba173d".to_string());
        let msg = Message::new(
            talk_id.clone(),
            user::Sub("auth0|valera".into()),
            "Lorem ipsum",
        );
        let last_msg = LastMessage::from(&msg);
        let expected = concat!(
            r#"<div class="last-message text-sm text-gray-500" id="lm-67dff625c469e51787ba173d">"#,
            "Lorem ipsum",
            r#"<i class="fa-solid fa-envelope text-green-600 ml-2"></i>"#,
            "</div>"
        );

        let actual = last_message(Some(&last_msg), &talk_id, None);

        assert_eq!(expected, actual.into_string())
    }

    #[test]
    fn should_render_not_owned_last_message() {
        let talk_id = talk::Id("67dff625c469e51787ba173d".to_string());
        let msg = Message::new(
            talk_id.clone(),
            user::Sub("auth0|valera".into()),
            "Lorem ipsum",
        );
        let last_msg = LastMessage::from(&msg);
        let expected = concat!(
            r#"<div class="last-message text-sm text-gray-500" id="lm-67dff625c469e51787ba173d">"#,
            "Lorem ipsum",
            r#"<i class="fa-solid fa-envelope text-green-600 ml-2"></i>"#,
            "</div>"
        );

        let actual = last_message(
            Some(&last_msg),
            &talk_id,
            Some(&user::Sub("google|jora".into())),
        );

        assert_eq!(expected, actual.into_string())
    }

    #[test]
    fn should_render_auth_subs_last_message() {
        let talk_id = talk::Id("67dff625c469e51787ba173d".to_string());
        let auth_sub = user::Sub("auth0|valera".into());
        let msg = Message::new(talk_id.clone(), auth_sub.clone(), "Lorem ipsum");
        let last_msg = LastMessage::from(&msg);
        let expected = concat!(
            r#"<div class="last-message text-sm text-gray-500" id="lm-67dff625c469e51787ba173d">"#,
            "Lorem ipsum",
            "</div>"
        );

        let actual = last_message(Some(&last_msg), &talk_id, Some(&auth_sub));

        assert_eq!(expected, actual.into_string())
    }

    #[test]
    fn should_render_trimmed_last_message_when_length_greater_than_25() {
        let talk_id = talk::Id("67dff625c469e51787ba173d".to_string());
        let auth_sub = user::Sub("auth0|valera".into());
        let msg = Message::new(
            talk_id.clone(),
            auth_sub.clone(),
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.",
        );
        let last_msg = LastMessage::from(&msg);
        let expected = concat!(
            r#"<div class="last-message text-sm text-gray-500" id="lm-67dff625c469e51787ba173d">"#,
            "Lorem ipsum dolor sit ame...",
            "</div>"
        );

        let actual = last_message(Some(&last_msg), &talk_id, Some(&auth_sub));

        assert_eq!(expected, actual.into_string())
    }

    #[test]
    fn should_return_message_id_as_attribute() {
        let id = message::Id("67dff625c469e51787ba173d".to_string());

        assert_eq!("m-67dff625c469e51787ba173d", id.attr())
    }

    #[test]
    fn should_return_message_id_as_target() {
        let id = message::Id("67dff625c469e51787ba173d".to_string());

        assert_eq!("#m-67dff625c469e51787ba173d", id.target())
    }

    #[test]
    fn should_render_edit_icon() {
        let talk_id = talk::Id("67dff625c469e51787ba173d".to_string());
        let auth_sub = user::Sub("auth0|valera".into());
        let msg = Message::new(talk_id.clone(), auth_sub.clone(), "Lorem ipsum");

        let expected = format!(
            "<i class=\"fa-pen fa-solid ml-2 text-green-700 cursor-pointer\" hx-get=\"/templates/messages/input/edit?message_id={}\" hx-target=\"#message-input\" hx-swap=\"outerHTML\"></i>",
            &msg.id
        );

        let actual = Icon::Edit(&msg).render();

        assert_eq!(expected, actual.into_string())
    }

    #[test]
    fn should_render_delete_icon() {
        let id = message::Id("67dff625c469e51787ba173d".to_string());

        let expected = "<i class=\"fa-trash-can fa-solid text-red-700 cursor-pointer\" hx-delete=\"/api/messages/67dff625c469e51787ba173d\" hx-target=\"#m-67dff625c469e51787ba173d\" hx-swap=\"outerHTML swap:200ms\"></i>";

        let actual = Icon::Delete(&id).render();

        assert_eq!(expected, actual.into_string())
    }

    #[test]
    fn should_render_sent_icon() {
        let expected =
            r#"<i class="fa-solid fa-check absolute bottom-1 right-1 text-white opacity-65"></i>"#;

        let actual = Icon::Sent.render();

        assert_eq!(expected, actual.into_string())
    }

    #[test]
    fn should_render_seen_icon() {
        let expected = r#"<i class="fa-solid fa-check absolute bottom-1 right-2.5 text-white opacity-65"></i>"#;

        let actual = Icon::Seen.render();

        assert_eq!(expected, actual.into_string())
    }
}
