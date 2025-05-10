use std::fmt::Display;
use std::rc::Rc;

use maud::{Markup, Render, html};

use crate::markup::IdExt;
use crate::message::markup::{MESSAGE_INPUT_TARGET, MESSAGE_LIST_ID, MESSAGE_LIST_TARGET};
use crate::talk::model::DetailsDto;
use crate::{auth, message, talk, user};

use super::handler::templates::GroupMemberDto;
use super::model::TalkDto;

impl Display for super::Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

const TALK_WINDOW_ID: &str = "talk-window";
pub const TALK_WINDOW_TARGET: &str = "#talk-window";

pub struct TalkWindow<'a> {
    auth_user: &'a auth::User,
    talks: Rc<[TalkDto]>,
    kind: talk::Kind,
}

// TODO: create separate markups for chat and group
impl<'a> TalkWindow<'a> {
    pub fn chats(auth_user: &'a auth::User, talks: &[TalkDto]) -> Self {
        Self {
            auth_user,
            talks: talks.into(),
            kind: talk::Kind::Chat,
        }
    }

    pub fn groups(auth_user: &'a auth::User, talks: &[TalkDto]) -> Self {
        Self {
            auth_user,
            talks: talks.into(),
            kind: talk::Kind::Group,
        }
    }

    fn get_talks(&self) -> &[TalkDto] {
        &self.talks
    }
}

impl Render for TalkWindow<'_> {
    fn render(&self) -> Markup {
        html! {
            div #(TALK_WINDOW_ID) ."flex flex-col h-full" {
                @match self.kind {
                    talk::Kind::Chat => {
                        (user::markup::Header(self.auth_user))
                        (user::markup::Search)
                    },
                    talk::Kind::Group => {
                        header ."text-center mb-4"{
                            h2.text-2xl { "Groups" }
                        }

                        a ."text-center text-white font-bold cursor-pointer"
                            ."bg-blue-500 hover:bg-blue-400 rounded"
                            ."py-2 px-4 mb-4"
                            hx-get="/templates/talks/group/create"
                            hx-target=(TALK_WINDOW_TARGET) { "Create group" }
                    },
                }

                (TalkList::new(self.get_talks()))
            }
        }
    }
}

struct TalkList(Rc<[TalkDto]>);

impl TalkList {
    fn new(talks: &[TalkDto]) -> Self {
        Self(talks.into())
    }

    fn get_talks(&self) -> &[TalkDto] {
        &self.0
    }
}

impl Render for TalkList {
    fn render(&self) -> Markup {
        html! {
            div #talk-list ."flex flex-col space-y-2 h-full overflow-y-auto"
                sse-swap="newTalk"
                hx-swap="beforeend"
                hx-target="#talk-list"
            {
                @for talk in self.get_talks() {
                    (talk)
                }
            }
        }
    }
}

struct Header<'a>(&'a TalkDto);

impl Render for Header<'_> {
    fn render(&self) -> Markup {
        let back_url = match self.0.details() {
            DetailsDto::Chat { .. } => "/tabs/chats",
            DetailsDto::Group { .. } => "/tabs/groups",
        };

        html! {
            header #recipient-header ."flex justify-between items-center" {
                a ."cursor-pointer border-2 border-red-500 text-red-500 px-4 py-2 rounded-2xl mr-4"
                    hx-get=(back_url)
                    hx-target="#tabs"
                    hx-swap="innerHTML" { "X" }
                ."flex text-2xl" {
                    @if let DetailsDto::Chat{ recipient, .. } = &self.0.details() {
                        (user::model::OnlineStatus::new(recipient.clone(), false))
                    }

                    (self.0.name())
                }
                (Icon::TalkControls)
            }
        }
    }
}

pub struct ActiveTalk<'a>(pub &'a auth::User, pub &'a TalkDto);

impl Render for ActiveTalk<'_> {
    fn render(&self) -> Markup {
        html! {
            (Header(self.1))

            #active-talk ."flex-grow overflow-auto mt-4 mb-4"
                hx-ext="ws"
                ws-connect={ "/ws/" (self.1.id()) }
            {
                div #(MESSAGE_LIST_ID) ."sticky flex flex-col-reverse overflow-auto h-full"
                    hx-get={ "/api/messages?limit=20&talk_id=" (self.1.id()) }
                    hx-trigger="load"
                    hx-target=(MESSAGE_LIST_TARGET) {}
            }

            (message::markup::InputBlank(self.1.id()))
            (TalkControls(&self.0, self.1))

            div .hidden
                hx-trigger="msg:afterUpdate from:body"
                hx-target=(MESSAGE_INPUT_TARGET)
                hx-swap="outerHTML"
                hx-get={"/templates/messages/input/blank?talk_id=" (self.1.id())} {}
        }
    }
}

const TALK_CONTROLS_ID: &str = "talk-controls";
pub const TALK_CONTROLS_TARGET: &str = "#talk-controls";

struct TalkControls<'a>(&'a auth::User, &'a TalkDto);

impl Render for TalkControls<'_> {
    fn render(&self) -> Markup {
        let controls_item_class = "text-lg py-3 cursor-pointer hover:bg-gray-300";

        let can_delete = match self.1.details() {
            DetailsDto::Chat { .. } => true,
            DetailsDto::Group { owner, .. } => owner.eq(self.0.sub()),
        };

        html! {
            div #(TALK_CONTROLS_ID) ."flex flex-row h-full w-full absolute top-0 left-0 invisible" {
                div ."talk-controls-overlay w-2/3 bg-gray-300 bg-opacity-50"
                    _="on click add .invisible to #talk-controls" {}

                div ."flex flex-col bg-white h-full w-1/3 py-4 text-center" {
                    div ."text-2xl py-3" { "Settings" }
                    @if can_delete {
                        div .(controls_item_class)
                            hx-delete={"/api/talks/" (self.1.id())} { "Delete talk" }
                    }
                }
            }
        }
    }
}

impl crate::markup::IdExt for talk::Id {
    fn attr(&self) -> String {
        format!("t-{}", self.0)
    }

    fn target(&self) -> String {
        format!("#t-{}", self.0)
    }
}

impl Render for TalkDto {
    fn render(&self) -> Markup {
        html! {
            div #(self.id().attr())
                ."talk-item px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center"
                hx-get={"/talks/" (self.id())}
                hx-target=(TALK_WINDOW_TARGET)
                hx-swap="innerHTML"
            {
                @if let DetailsDto::Chat{recipient, ..} = &self.details() {
                    (user::model::OnlineStatus::new(recipient.clone(), false))
                }
                img ."w-8 h-8 rounded-full"
                    src=(self.picture()) alt="Talk avatar" {}

                span ."talk-recipient font-bold mx-2" { (self.name()) }

                div ."flex-grow text-right truncate"
                    sse-swap={"newMessage:"(self.id())}
                    hx-target={"#lm-"(self.id())}
                {
                    ({
                        let sender = match &self.details() {
                            DetailsDto::Chat { sender, .. }
                            | DetailsDto::Group { sender, .. } => sender,
                        };

                        message::markup::last_message(self.last_message(), self.id(), Some(sender))
                    })
                }
            }
        }
    }
}

pub struct CreateGroupForm<'a> {
    auth_user: &'a auth::User,
    members: &'a [GroupMemberDto],
}

impl<'a> CreateGroupForm<'a> {
    pub const fn new(auth_user: &'a auth::User, members: &'a [GroupMemberDto]) -> Self {
        Self { auth_user, members }
    }
}

impl Render for CreateGroupForm<'_> {
    fn render(&self) -> Markup {
        html! {
            header ."text-center mb-4"{
                h2.text-2xl { "Create group" }
            }

            form ."flex flex-col h-full"
                hx-post="/api/talks"
                hx-target=(TALK_WINDOW_TARGET)
                hx-ext="json-enc" {
                input type="hidden" name="kind" value="group" {}
                input ."mb-2 w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none"
                    type="text" name="name" placeholder="Group name" {}

                fieldset ."sticky flex flex-col overflow-auto"
                    ."space-y-2 border border-gray-300 rounded-md px-3 pb-3 h-3/4 mb-4"
                {
                    legend { "Select at least two members" }
                    input type="hidden" name="members" value=(self.auth_user.sub()) {}
                    @for m in self.members {
                        label ."flex items-center justify-between px-3 py-2"
                            ."rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer"
                        {
                            div ."member-details flex items-center" {
                                img class="w-9 h-9 rounded-full float-left mr-2"
                                    src=(m.picture())
                                    alt="User avatar" {}
                                span ."font-bold mx-2" { (m.name()) }
                            }
                            input type="checkbox" name="members" value=(m.sub()) {}
                        }
                    }
                }
                input type="submit" value="Create"
                    ."text-white px-4 py-2 rounded-md w-full"
                    ."cursor-pointer bg-blue-600 hover:bg-blue-700"
                    hx-disabled-elt="this" {}
            }
        }
    }
}

pub enum Icon {
    TalkControls,
    Unseen,
}

impl Render for Icon {
    fn render(&self) -> Markup {
        html! {
            @match self {
                Self::TalkControls => {
                    i ."fa-solid fa-bars text-2xl cursor-pointer"
                        _={ "on click toggle .invisible on " (TALK_CONTROLS_TARGET) } {}
                },
                Self::Unseen => i ."fa-solid fa-envelope text-green-600 ml-2" {}
            }

        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        message::model::LastMessage,
        user::{Nickname, Sub},
    };

    use super::*;

    #[test]
    fn should_render_chat_talk_window() {
        let expected = concat!(
            r#"<div class="flex flex-col h-full" id="talk-window">"#,
            r#"<header class="flex items-center place-content-center mb-4" id="user-header">"#,
            r#"<img class="w-12 h-12 rounded-full mr-3" src="jora://picture" alt="User avatar"></img>"#,
            r#"<h2 class="text-2xl">Jora</h2>"#,
            "</header>",
            r##"<input class="mb-4 w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none" type="search" name="nickname" placeholder="Search users..." autocomplete="off" hx-post="/api/users/search" hx-trigger="input changed delay:500ms" hx-target="#search-results" "##,
            r#"_="on keyup
                if the event's key is 'Escape'
                    set value of me to ''
                    remove children of #search-results">"#,
            "</input>",
            r#"<div class="relative" id="search-results"></div>"#,
            r##"<div class="flex flex-col space-y-2 h-full overflow-y-auto" id="talk-list" sse-swap="newTalk" hx-swap="beforeend" hx-target="#talk-list">"##,
            r##"<div class="talk-item px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center" id="t-680d0fa361f9e3c2a1b25c4f" hx-get="/talks/680d0fa361f9e3c2a1b25c4f" hx-target="#talk-window" hx-swap="innerHTML">"##,
            r##"<div sse-swap="onlineStatusChange:valera" hx-target="#os-valera" hx-swap="outerHTML">"##,
            r#"<i class="fa-regular fa-circle text-green-600 mr-2 text-sm" id="os-valera"></i>"#,
            "</div>",
            r#"<img class="w-8 h-8 rounded-full" src="talk1://picture" alt="Talk avatar"></img>"#,
            r#"<span class="talk-recipient font-bold mx-2">Valera</span>"#,
            r##"<div class="flex-grow text-right truncate" sse-swap="newMessage:680d0fa361f9e3c2a1b25c4f" hx-target="#lm-680d0fa361f9e3c2a1b25c4f">"##,
            r#"<div class="last-message text-sm text-gray-500" id="lm-680d0fa361f9e3c2a1b25c4f">LGTM!</div>"#,
            "</div>",
            "</div>",
            r##"<div class="talk-item px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center" id="t-680d0fa361f9e3c2a1b25c4g" hx-get="/talks/680d0fa361f9e3c2a1b25c4g" hx-target="#talk-window" hx-swap="innerHTML">"##,
            r##"<div sse-swap="onlineStatusChange:igor" hx-target="#os-igor" hx-swap="outerHTML">"##,
            r#"<i class="fa-regular fa-circle text-green-600 mr-2 text-sm" id="os-igor"></i>"#,
            "</div>",
            r#"<img class="w-8 h-8 rounded-full" src="talk2://picture" alt="Talk avatar"></img>"#,
            r#"<span class="talk-recipient font-bold mx-2">Igor</span>"#,
            r##"<div class="flex-grow text-right truncate" sse-swap="newMessage:680d0fa361f9e3c2a1b25c4g" hx-target="#lm-680d0fa361f9e3c2a1b25c4g">"##,
            r#"<div class="last-message text-sm text-gray-500" id="lm-680d0fa361f9e3c2a1b25c4g">What's up?</div>"#,
            "</div>",
            "</div>",
            "</div>",
            "</div>"
        );

        let auth_user = auth::User::new(
            Sub::from("jora"),
            Nickname::from("jora"),
            "Jora",
            "jora://picture",
        );
        let talks = [
            TalkDto::new(
                talk::Id("680d0fa361f9e3c2a1b25c4f".into()),
                "talk1://picture",
                "Valera",
                DetailsDto::Chat {
                    sender: Sub::from("github|jora"),
                    recipient: Sub::from("google|valera"),
                },
                Some(LastMessage::new(
                    message::Id::random(),
                    "LGTM!",
                    Sub::from("github|jora"),
                    chrono::Utc::now().timestamp(),
                    true,
                )),
            ),
            TalkDto::new(
                talk::Id("680d0fa361f9e3c2a1b25c4g".into()),
                "talk2://picture",
                "Igor",
                DetailsDto::Chat {
                    sender: Sub::from("github|jora"),
                    recipient: Sub::from("google|igor"),
                },
                Some(LastMessage::new(
                    message::Id::random(),
                    "What's up?",
                    Sub::from("github|igor"),
                    chrono::Utc::now().timestamp(),
                    true,
                )),
            ),
        ];

        let actual = TalkWindow::chats(&auth_user, &talks).render().into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_group_talk_window() {
        let expected = html! {
            div class="flex flex-col h-full" id="talk-window" {
                header class="text-center mb-4" {
                    h2 class="text-2xl" { "Groups" }
                }
                a class="text-center text-white font-bold cursor-pointer bg-blue-500 hover:bg-blue-400 py-2 px-4 rounded"
                    hx-get="/templates/talks/group/create" hx-target="#talk-window"
                {
                    "Create group"
                }
                div class="flex flex-col space-y-2 h-full overflow-y-auto" id="talk-list" sse-swap="newTalk" hx-swap="beforeend" hx-target="#talk-list" {
                    div class="talk-item px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center" id="t-680d0fa361f9e3c2a1b25c4f" hx-get="/talks/680d0fa361f9e3c2a1b25c4f" hx-target="#talk-window" hx-swap="innerHTML" {
                        img class="w-8 h-8 rounded-full" src="talk1://picture" alt="Talk avatar" {}
                        span class="talk-recipient font-bold mx-2" { "Que pasa?" }
                        div class="flex-grow text-right truncate" sse-swap="newMessage:680d0fa361f9e3c2a1b25c4f" hx-target="#lm-680d0fa361f9e3c2a1b25c4f" {
                            div class="last-message text-sm text-gray-500" id="lm-680d0fa361f9e3c2a1b25c4f" {
                                "LGTM!"
                            }
                        }
                    }
                    div class="talk-item px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center" id="t-680d0fa361f9e3c2a1b25c4g" hx-get="/talks/680d0fa361f9e3c2a1b25c4g" hx-target="#talk-window" hx-swap="innerHTML" {
                        img class="w-8 h-8 rounded-full" src="talk2://picture" alt="Talk avatar" {}
                        span class="talk-recipient font-bold mx-2" { "Wigas" }
                        div class="flex-grow text-right truncate" sse-swap="newMessage:680d0fa361f9e3c2a1b25c4g" hx-target="#lm-680d0fa361f9e3c2a1b25c4g" {
                            div class="last-message text-sm text-gray-500" id="lm-680d0fa361f9e3c2a1b25c4g" {
                                "What's up?"
                            }
                        }
                    }
                    div class="talk-item px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center" id="t-680d0fa361f9e3c2a1b25c4h" hx-get="/talks/680d0fa361f9e3c2a1b25c4h" hx-target="#talk-window" hx-swap="innerHTML" {
                        img class="w-8 h-8 rounded-full" src="talk3://picture" alt="Talk avatar" {}
                        span class="talk-recipient font-bold mx-2" { "Red bull" }
                        div class="flex-grow text-right truncate" sse-swap="newMessage:680d0fa361f9e3c2a1b25c4h" hx-target="#lm-680d0fa361f9e3c2a1b25c4h" {
                            div class="last-message text-sm text-gray-500" id="lm-680d0fa361f9e3c2a1b25c4h" {
                                "High energy!"
                            }
                        }
                    }
                    div class="talk-item px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center" id="t-680d0fa361f9e3c2a1b25c4k" hx-get="/talks/680d0fa361f9e3c2a1b25c4k" hx-target="#talk-window" hx-swap="innerHTML" {
                        img class="w-8 h-8 rounded-full" src="talk4://picture" alt="Talk avatar" {}
                        span class="talk-recipient font-bold mx-2" { "Tuners IO" }
                        div class="flex-grow text-right truncate" sse-swap="newMessage:680d0fa361f9e3c2a1b25c4k" hx-target="#lm-680d0fa361f9e3c2a1b25c4k" {
                            div class="last-message text-sm text-gray-500" id="lm-680d0fa361f9e3c2a1b25c4k" {
                                "1000 HP"
                                i class="fa-solid fa-envelope text-green-600 ml-2" {}
                            }
                        }
                    }
                }
            }
        }.into_string();

        let auth_user = auth::User::new(
            Sub::from("jora"),
            Nickname::from("jora"),
            "Jora",
            "jora://picture",
        );
        let talks = [
            TalkDto::new(
                talk::Id("680d0fa361f9e3c2a1b25c4f".into()),
                "talk1://picture",
                "Que pasa?",
                DetailsDto::Group {
                    owner: auth_user.sub().clone(),
                    sender: auth_user.sub().clone(),
                },
                Some(LastMessage::new(
                    message::Id::random(),
                    "LGTM!",
                    auth_user.sub().clone(),
                    chrono::Utc::now().timestamp(),
                    true,
                )),
            ),
            TalkDto::new(
                talk::Id("680d0fa361f9e3c2a1b25c4g".into()),
                "talk2://picture",
                "Wigas",
                DetailsDto::Group {
                    owner: auth_user.sub().clone(),
                    sender: auth_user.sub().clone(),
                },
                Some(LastMessage::new(
                    message::Id::random(),
                    "What's up?",
                    Sub::from("github|igor"),
                    chrono::Utc::now().timestamp(),
                    true,
                )),
            ),
            TalkDto::new(
                talk::Id("680d0fa361f9e3c2a1b25c4h".into()),
                "talk3://picture",
                "Red bull",
                DetailsDto::Group {
                    owner: auth_user.sub().clone(),
                    sender: auth_user.sub().clone(),
                },
                Some(LastMessage::new(
                    message::Id::random(),
                    "High energy!",
                    auth_user.sub().clone(),
                    chrono::Utc::now().timestamp(),
                    false,
                )),
            ),
            TalkDto::new(
                talk::Id("680d0fa361f9e3c2a1b25c4k".into()),
                "talk4://picture",
                "Tuners IO",
                DetailsDto::Group {
                    owner: auth_user.sub().clone(),
                    sender: auth_user.sub().clone(),
                },
                Some(LastMessage::new(
                    message::Id::random(),
                    "1000 HP",
                    Sub::from("github|radu"),
                    chrono::Utc::now().timestamp(),
                    false,
                )),
            ),
        ];

        let actual = TalkWindow::groups(&auth_user, &talks)
            .render()
            .into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_header() {
        let expected = concat!(
            r#"<header class="flex justify-between items-center" id="recipient-header">"#,
            r##"<a class="cursor-pointer border-2 border-red-500 text-red-500 px-4 py-2 rounded-2xl mr-4" hx-get="/tabs/chats" hx-target="#tabs" hx-swap="innerHTML">X</a>"##,
            r#"<div class="flex text-2xl">"#,
            r##"<div sse-swap="onlineStatusChange:valera" hx-target="#os-valera" hx-swap="outerHTML">"##,
            r#"<i class="fa-regular fa-circle text-green-600 mr-2 text-sm" id="os-valera"></i>"#,
            "</div>",
            "Wiggas",
            "</div>",
            r#"<i class="fa-solid fa-bars text-2xl cursor-pointer" _="on click toggle .invisible on #talk-controls"></i>"#,
            "</header>"
        );

        let t = TalkDto::new(
            talk::Id("680d0fa361f9e3c2a1b25c4f".into()),
            "talk://picture",
            "Wiggas",
            DetailsDto::Chat {
                sender: Sub::from("github|jora"),
                recipient: Sub::from("google|valera"),
            },
            Some(LastMessage::new(
                message::Id::random(),
                "LGTM!",
                Sub::from("github|jora"),
                chrono::Utc::now().timestamp(),
                true,
            )),
        );

        let actual = Header(&t).render().into_string();

        assert_eq!(expected, actual);
    }

    #[test]
    fn should_render_active_talk() {
        let expected = concat!(
            r#"<header class="flex justify-between items-center" id="recipient-header">"#,
            r##"<a class="cursor-pointer border-2 border-red-500 text-red-500 px-4 py-2 rounded-2xl mr-4" hx-get="/tabs/chats" hx-target="#tabs" hx-swap="innerHTML">X</a>"##,
            r#"<div class="flex text-2xl">"#,
            r##"<div sse-swap="onlineStatusChange:valera" hx-target="#os-valera" hx-swap="outerHTML">"##,
            r#"<i class="fa-regular fa-circle text-green-600 mr-2 text-sm" id="os-valera"></i>"#,
            "</div>",
            "Wiggas",
            "</div>",
            r#"<i class="fa-solid fa-bars text-2xl cursor-pointer" _="on click toggle .invisible on #talk-controls"></i>"#,
            "</header>",
            r#"<div class="flex-grow overflow-auto mt-4 mb-4" id="active-talk" hx-ext="ws" ws-connect="/ws/680d0fa361f9e3c2a1b25c4f">"#,
            r##"<div class="sticky flex flex-col-reverse overflow-auto h-full" id="message-list" hx-get="/api/messages?limit=20&amp;talk_id=680d0fa361f9e3c2a1b25c4f" hx-trigger="load" hx-target="#message-list">"##,
            "</div>",
            "</div>",
            r##"<form class="border-gray-200 flex mb-3" id="message-input" hx-post="/api/messages" hx-target="#message-list" hx-swap="afterbegin" "##,
            r#"_="on htmx:afterRequest reset() me
            on htmx:afterRequest go to the bottom of the #message-list">"#,
            r#"<input type="hidden" name="talk_id" value="680d0fa361f9e3c2a1b25c4f"></input>"#,
            r#"<input class="border border-gray-300 rounded-l-md p-2 flex-1 focus:outline-none" type="text" name="text" placeholder="Type your message..." autocomplete="off" hx-disabled-elt="this" _="on keyup if the event's key is 'Escape' set value of me to ''"></input>"#,
            r#"<input class="bg-blue-600 text-white px-4 rounded-r-md cursor-pointer hover:bg-blue-700" hx-disabled-elt="this" type="submit" value="Send"></input>"#,
            "</form>",
            r#"<div class="flex flex-row h-full w-full absolute top-0 left-0 invisible" id="talk-controls">"#,
            r#"<div class="talk-controls-overlay w-2/3 bg-gray-300 bg-opacity-50" _="on click add .invisible to #talk-controls"></div>"#,
            r#"<div class="flex flex-col bg-white h-full w-1/3 py-4 text-center">"#,
            r#"<div class="text-2xl py-3">Settings</div>"#,
            r#"<div class="text-lg py-3 cursor-pointer hover:bg-gray-300" hx-delete="/api/talks/680d0fa361f9e3c2a1b25c4f">Delete talk</div>"#,
            "</div>",
            "</div>",
            r##"<div class="hidden" hx-trigger="msg:afterUpdate from:body" hx-target="#message-input" hx-swap="outerHTML" hx-get="/templates/messages/input/blank?talk_id=680d0fa361f9e3c2a1b25c4f"></div>"##
        );

        let t = TalkDto::new(
            talk::Id("680d0fa361f9e3c2a1b25c4f".into()),
            "talk://picture",
            "Wiggas",
            DetailsDto::Chat {
                sender: Sub::from("github|jora"),
                recipient: Sub::from("google|valera"),
            },
            Some(LastMessage::new(
                message::Id::random(),
                "LGTM!",
                Sub::from("github|jora"),
                chrono::Utc::now().timestamp(),
                true,
            )),
        );

        let auth_user = auth::User::new(
            Sub::from("jora"),
            Nickname::from("jora"),
            "Jora",
            "jora://picture",
        );
        let actual = ActiveTalk(&auth_user, &t).render().into_string();

        assert_eq!(expected, actual);
    }

    #[test]
    fn should_render_talk_controls() {
        let expected = concat!(
            r#"<div class="flex flex-row h-full w-full absolute top-0 left-0 invisible" id="talk-controls">"#,
            r#"<div class="talk-controls-overlay w-2/3 bg-gray-300 bg-opacity-50" _="on click add .invisible to #talk-controls"></div>"#,
            r#"<div class="flex flex-col bg-white h-full w-1/3 py-4 text-center">"#,
            r#"<div class="text-2xl py-3">Settings</div>"#,
            r#"<div class="text-lg py-3 cursor-pointer hover:bg-gray-300" hx-delete="/api/talks/680d10a4042fe1d7f2d6138b">Delete talk</div>"#,
            "</div>",
            "</div>"
        );

        let auth_user = auth::User::new(
            Sub::from("jora"),
            Nickname::from("jora"),
            "Jora",
            "jora://picture",
        );
        let t = TalkDto::new(
            talk::Id("680d10a4042fe1d7f2d6138b".into()),
            "talk://picture",
            "Wiggas",
            DetailsDto::Chat {
                sender: Sub::from("github|jora"),
                recipient: Sub::from("google|valera"),
            },
            Some(LastMessage::new(
                message::Id::random(),
                "LGTM!",
                Sub::from("github|jora"),
                chrono::Utc::now().timestamp(),
                true,
            )),
        );

        let actual = TalkControls(&auth_user, &t).render().into_string();

        assert_eq!(expected, actual);
    }

    #[test]
    fn should_render_chat_talk_dto() {
        let expected = concat!(
            r##"<div class="talk-item px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center" id="t-680d0fa361f9e3c2a1b25c4f" hx-get="/talks/680d0fa361f9e3c2a1b25c4f" hx-target="#talk-window" hx-swap="innerHTML">"##,
            r##"<div sse-swap="onlineStatusChange:valera" hx-target="#os-valera" hx-swap="outerHTML">"##,
            r#"<i class="fa-regular fa-circle text-green-600 mr-2 text-sm" id="os-valera"></i>"#,
            "</div>",
            r#"<img class="w-8 h-8 rounded-full" src="talk://picture" alt="Talk avatar"></img>"#,
            r#"<span class="talk-recipient font-bold mx-2">Wiggas</span>"#,
            r##"<div class="flex-grow text-right truncate" sse-swap="newMessage:680d0fa361f9e3c2a1b25c4f" hx-target="#lm-680d0fa361f9e3c2a1b25c4f">"##,
            r#"<div class="last-message text-sm text-gray-500" id="lm-680d0fa361f9e3c2a1b25c4f">LGTM!</div>"#,
            "</div>",
            "</div>"
        );

        let actual = TalkDto::new(
            talk::Id("680d0fa361f9e3c2a1b25c4f".into()),
            "talk://picture",
            "Wiggas",
            DetailsDto::Chat {
                sender: Sub::from("github|jora"),
                recipient: Sub::from("google|valera"),
            },
            Some(LastMessage::new(
                message::Id::random(),
                "LGTM!",
                Sub::from("github|jora"),
                chrono::Utc::now().timestamp(),
                true,
            )),
        )
        .render()
        .into_string();

        assert_eq!(expected, actual);
    }

    #[test]
    fn should_render_talk_controls_icon() {
        let expected = r#"<i class="fa-solid fa-bars text-2xl cursor-pointer" _="on click toggle .invisible on #talk-controls"></i>"#;

        let actual = Icon::TalkControls.render().into_string();

        assert_eq!(expected, actual);
    }

    #[test]
    fn should_render_unseen_icon() {
        let expected = r#"<i class="fa-solid fa-envelope text-green-600 ml-2"></i>"#;

        let actual = Icon::Unseen.render().into_string();

        assert_eq!(expected, actual);
    }
}
