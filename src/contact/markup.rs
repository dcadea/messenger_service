use core::fmt;
use std::fmt::Display;

use maud::{Markup, Render, html};

use crate::{
    auth,
    contact::{self, Status, Transition},
    user::model::UserDto,
};

use super::model::ContactDto;

impl Display for super::Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

pub struct ContactInfos<'a> {
    pub auth_user: &'a auth::User,
    pub contact_infos: &'a [(ContactDto, UserDto)],
}

impl<'a> ContactInfos<'a> {
    pub const fn new(
        auth_user: &'a auth::User,
        contact_infos: &'a [(ContactDto, UserDto)],
    ) -> Self {
        Self {
            auth_user,
            contact_infos,
        }
    }
}

const CONTACT_ITEM_CLASS: &str =
    "px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center";

impl Render for ContactInfos<'_> {
    fn render(&self) -> maud::Markup {
        html! {
            header ."text-center mb-4"{
                h2.text-2xl { "Contacts" }
            }
            ul ."flex flex-col space-y-2" {
                @for (c, u) in self.contact_infos {
                    li .(CONTACT_ITEM_CLASS) {
                        img ."w-9 h-9 rounded-full float-left mr-2"
                            src=(u.picture())
                            alt="User avatar" {}
                        (u.name())

                        (Icons::new(c.id(), c.status(), self.auth_user))
                    }
                }
            }
        }
    }
}

pub struct Icons<'a> {
    contact_id: &'a contact::Id,
    status: &'a Status,
    auth_user: &'a auth::User,
}

impl<'a> Icons<'a> {
    pub const fn new(
        contact_id: &'a contact::Id,
        status: &'a Status,
        auth_user: &'a auth::User,
    ) -> Self {
        Self {
            contact_id,
            status,
            auth_user,
        }
    }
}

impl Render for Icons<'_> {
    fn render(&self) -> Markup {
        let c_id = self.contact_id;
        let auth_id = self.auth_user.id();

        html! {
            div #{"ci-status-" (c_id)}
                ."grow text-right"
            {
                @match self.status {
                    Status::Pending { initiator } => {
                        @if initiator.eq(auth_id) {
                            (Icon::Pending)
                        } @else {
                            (Icon::Accept(c_id))
                            (Icon::Reject(c_id))
                        }
                    },
                    Status::Accepted => (Icon::Block(c_id)),
                    Status::Rejected => (Icon::Rejected),
                    Status::Blocked { initiator } => {
                        @if initiator.eq(auth_id) {
                            "Blocked"
                            (Icon::Unblock(c_id))
                        } @else {
                            "Blocked you"
                        }
                    },
                }
            }
        }
    }
}

enum Icon<'a> {
    Pending,
    Accept(&'a contact::Id),
    Reject(&'a contact::Id),
    Block(&'a contact::Id),
    Unblock(&'a contact::Id),
    Rejected,
}

impl Render for Icon<'_> {
    fn render(&self) -> Markup {
        let hx_icon = |id: &contact::Id, action: Transition, i_class: &str| {
            html! {
                i .{"fa-solid " (i_class) " cursor-pointer"}
                    hx-target={"#ci-status-" (id)}
                    hx-put={"/api/contacts/" (id) "/" (action)} {}
            }
        };

        html! {
            @match self {
                Self::Pending => {
                    span.text-blue-500 {
                        i ."fa-solid fa-hourglass-half mr-2" {}
                        "Pending action"
                    }
                },
                Self::Accept(id) => {
                    (hx_icon(id, Transition::Accept, "fa-check text-2xl text-green-600"))
                },
                Self::Reject(id) => {
                    (hx_icon(id, Transition::Reject, "fa-xmark ml-3 text-2xl text-red-500"))
                },
                Self::Block(id) => {
                    (hx_icon(id, Transition::Block, "fa-ban ml-3 text-2xl"))
                },
                Self::Unblock(id) => {
                    (hx_icon(id, Transition::Unblock, "fa-lock-open ml-3 text-green-600 text-xl"))
                },
                Self::Rejected => {
                    span.text-red-500 {
                        i ."fa-solid fa-xmark mr-2" {}
                        "Request rejected"
                    }
                },
            }
        }
    }
}

impl fmt::Display for Transition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Accept => write!(f, "accept"),
            Self::Reject => write!(f, "reject"),
            Self::Block => write!(f, "block"),
            Self::Unblock => write!(f, "unblock"),
        }
    }
}
