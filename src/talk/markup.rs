use std::rc::Rc;

use maud::{Markup, Render, html};

use crate::markup::IdExt;
use crate::message::markup::{MESSAGE_INPUT_TARGET, MESSAGE_LIST_ID, MESSAGE_LIST_TARGET};
use crate::talk::model::DetailsDto;
use crate::{auth, message, talk, user};

use super::model::TalkDto;

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
                        // if self.kind == Group
                        //     allow creation of group
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
        html! {
            header #recipient-header ."flex justify-between items-center" {
                a ."cursor-pointer border-2 border-red-500 text-red-500 px-4 py-2 rounded-2xl mr-4"
                    href="/" { "X" }
                ."flex text-2xl" {
                    @if let DetailsDto::Chat{recipient, ..} = &self.0.details {
                        (user::model::OnlineStatus::new(recipient.clone(), false))
                    }

                    (self.0.name)
                }
                (Icon::TalkControls)
            }
        }
    }
}

pub struct ActiveTalk<'a>(pub &'a TalkDto);

impl Render for ActiveTalk<'_> {
    fn render(&self) -> Markup {
        html! {
            (Header(self.0))

            div #active-talk ."flex-grow overflow-auto mt-4 mb-4"
                hx-ext="ws"
                ws-connect={ "/ws/" (self.0.id) }
            {
                div #(MESSAGE_LIST_ID) ."sticky flex flex-col-reverse overflow-auto h-full"
                    hx-get={ "/api/messages?limit=20&talk_id=" (self.0.id) }
                    hx-trigger="load"
                    hx-target=(MESSAGE_LIST_TARGET) {}
            }

            (message::markup::InputBlank(&self.0.id))
            (TalkControls(&self.0.id))

            div .hidden
                hx-trigger="msg:afterUpdate from:body"
                hx-target=(MESSAGE_INPUT_TARGET)
                hx-swap="outerHTML"
                hx-get={"/templates/messages/input/blank?talk_id=" (self.0.id)} {}
        }
    }
}

struct TalkControls<'a>(&'a talk::Id);

impl Render for TalkControls<'_> {
    fn render(&self) -> Markup {
        let controls_item_class = "text-lg py-3 cursor-pointer hover:bg-gray-300";

        html! {
            div #talk-controls ."flex flex-row h-full w-full absolute top-0 left-0 invisible" {
                div ."talk-controls-overlay w-2/3 bg-gray-300 bg-opacity-50"
                    _="on click add .invisible to #talk-controls" {}

                div ."flex flex-col bg-white h-full w-1/3 py-4 text-center" {
                    div ."text-2xl py-3" { "Settings" }
                    div .(controls_item_class)
                        hx-delete={"/api/talks/" (self.0)} { "Delete talk" }
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
            div #(self.id.attr())
                ."talk-item px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center"
                hx-get={"/talks/" (self.id)}
                hx-target=(TALK_WINDOW_TARGET)
                hx-swap="innerHTML"
            {
                @if let DetailsDto::Chat{recipient, ..} = &self.details {
                    (user::model::OnlineStatus::new(recipient.clone(), false))
                }
                img ."w-8 h-8 rounded-full"
                    src=(self.picture) alt="Talk avatar" {}

                span ."talk-recipient font-bold mx-2" { (self.name) }

                div ."flex-grow text-right truncate"
                    sse-swap={"newMessage:"(self.id)}
                    hx-target={"#lm-"(self.id)}
                {
                    ({
                        let sender = match &self.details {
                            DetailsDto::Chat{sender, ..} => Some(sender),
                            DetailsDto::Group => None,
                        };

                        message::markup::last_message(self.last_message.as_ref(), &self.id, sender)
                    })
                }
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
                        _="on click toggle .invisible on #talk-controls" {}
                },
                Self::Unseen => i ."fa-solid fa-envelope text-green-600 ml-2" {}
            }

        }
    }
}
