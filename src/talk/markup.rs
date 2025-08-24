use std::fmt::Display;
use std::rc::Rc;

use maud::{Markup, Render, html};
use messenger_service::AsStr;

use crate::markup::IdExt;
use crate::message::markup::{MESSAGE_INPUT_TARGET, MESSAGE_LIST_ID, MESSAGE_LIST_TARGET};
use crate::talk::Kind;
use crate::talk::model::DetailsDto;
use crate::{auth, message, talk, user};

use super::handler::templates::GroupMemberDto;
use super::model::TalkDto;

impl Display for super::Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

const TALK_WINDOW_ID: &str = "talk-window";
pub const TALK_WINDOW_TARGET: &str = "#talk-window";

pub struct TalkWindow<'a> {
    auth_user: &'a auth::User,
    talks: Rc<[TalkDto]>,
    kind: talk::Kind,
}

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
                        (user::model::OnlineStatus::from_ref(recipient, false))
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
            DetailsDto::Group { owner, .. } => owner.eq(self.0.id()),
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
        let kind = match self.details() {
            DetailsDto::Chat { .. } => Kind::Chat,
            DetailsDto::Group { .. } => Kind::Group,
        };

        html! {
            div #(self.id().attr())
                ."talk-item px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center"
                hx-get={"/talks/" (self.id()) "?kind=" (kind.as_str())}
                hx-target=(TALK_WINDOW_TARGET)
                hx-swap="innerHTML"
            {
                @if let DetailsDto::Chat{recipient, ..} = &self.details() {
                    (user::model::OnlineStatus::from_ref(recipient, false))
                }
                img ."w-8 h-8 rounded-full"
                    src=(self.picture().as_str()) alt="Talk avatar" {}

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
                    input type="hidden" name="members" value=(self.auth_user.id()) {}
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
                            input type="checkbox" name="members" value=(m.user_id()) {}
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
