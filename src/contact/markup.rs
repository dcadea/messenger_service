use core::fmt;
use std::fmt::Display;

use maud::{Markup, Render, html};

use crate::{
    auth,
    contact::{self, Status, Transition},
    user::model::UserInfo,
};

use super::model::ContactDto;

impl Display for super::Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct ContactInfos<'a> {
    pub auth_user: &'a auth::User,
    pub contact_infos: &'a [(ContactDto, UserInfo)],
}

impl<'a> ContactInfos<'a> {
    pub const fn new(
        auth_user: &'a auth::User,
        contact_infos: &'a [(ContactDto, UserInfo)],
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
                @for (c, ui) in self.contact_infos {
                    li .(CONTACT_ITEM_CLASS) {
                        img ."w-9 h-9 rounded-full float-left mr-2"
                            src=(ui.picture())
                            alt="User avatar" {}
                        (ui.name())

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
        let auth_sub = self.auth_user.sub();

        html! {
            div #{"ci-status-" (c_id)}
                ."grow text-right"
            {
                @match self.status {
                    Status::Pending { initiator } => {
                        @if initiator.eq(auth_sub) {
                            (Icon::Pending)
                        } @else {
                            (Icon::Accept(c_id))
                            (Icon::Reject(c_id))
                        }
                    },
                    Status::Accepted => (Icon::Block(c_id)),
                    Status::Rejected => (Icon::Rejected),
                    Status::Blocked { initiator } => {
                        @if initiator.eq(auth_sub) {
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

#[cfg(test)]
mod test {
    use crate::user::{Email, Nickname, Picture, Sub};

    use super::*;

    #[test]
    fn should_render_contact_infos() {
        let expected = html! {
            header class="text-center mb-4"{
                h2 class="text-2xl" { "Contacts" }
            }
            ul class="flex flex-col space-y-2" {
                li class="px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center" {
                    img class="w-9 h-9 rounded-full float-left mr-2"
                        src="jora://picture"
                        alt="User avatar" {}
                    "Jora"

                    div class="grow text-right" id="ci-status-680d045617d7edcb069071d8" {
                        i class="fa-solid fa-ban ml-3 text-2xl cursor-pointer"
                            hx-target="#ci-status-680d045617d7edcb069071d8"
                            hx-put="/api/contacts/680d045617d7edcb069071d8/block" {}
                    }
                }
                li class="px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center" {
                    img class="w-9 h-9 rounded-full float-left mr-2"
                        src="igor://picture"
                        alt="User avatar" {}
                    "Igor"

                    div class="grow text-right" id="ci-status-680d045617d7edcb069071d9" {
                        span class="text-red-500" {
                            i class="fa-solid fa-xmark mr-2" {}
                            "Request rejected"
                        }
                    }
                }
                li class="px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center" {
                    img class="w-9 h-9 rounded-full float-left mr-2"
                        src="radu://picture"
                        alt="User avatar" {}
                    "Radu"

                    div class="grow text-right" id="ci-status-680d045617d7edcb069071da" {
                        span class="text-blue-500" {
                            i class="fa-solid fa-hourglass-half mr-2" {}
                            "Pending action"
                        }
                    }
                }
                li class="px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center" {
                    img class="w-9 h-9 rounded-full float-left mr-2"
                        src="gicu://picture"
                        alt="User avatar" {}
                    "Gicu"

                    div class="grow text-right" id="ci-status-680d045617d7edcb069071db" {
                        i class="fa-solid fa-check text-2xl text-green-600 cursor-pointer"
                            hx-target="#ci-status-680d045617d7edcb069071db"
                            hx-put="/api/contacts/680d045617d7edcb069071db/accept" {}
                        i class="fa-solid fa-xmark ml-3 text-2xl text-red-500 cursor-pointer"
                            hx-target="#ci-status-680d045617d7edcb069071db"
                            hx-put="/api/contacts/680d045617d7edcb069071db/reject" {}
                    }
                }
                li class="px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center" {
                    img class="w-9 h-9 rounded-full float-left mr-2"
                        src="toha://picture"
                        alt="User avatar" {}
                    "Toha"

                    div class="grow text-right" id="ci-status-680d045617d7edcb069071dc" {
                        "Blocked"
                        i class="fa-solid fa-lock-open ml-3 text-green-600 text-xl cursor-pointer"
                            hx-target="#ci-status-680d045617d7edcb069071dc"
                            hx-put="/api/contacts/680d045617d7edcb069071dc/unblock" {}
                    }
                }
                li class="px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center" {
                    img class="w-9 h-9 rounded-full float-left mr-2"
                        src="alex://picture"
                        alt="User avatar" {}
                    "Alex"

                    div class="grow text-right" id="ci-status-680d045617d7edcb069071dd" {
                        "Blocked you"
                    }
                }
            }
        }.into_string();

        let auth_user = auth::User::new(
            Sub::from("valera"),
            Nickname::from("valera"),
            "Valera",
            Picture::parse("valera://picture").unwrap(),
        );

        let contact_infos = [
            (
                ContactDto::new(
                    contact::Id("680d045617d7edcb069071d8".into()),
                    Sub::from("jora"),
                    contact::Status::Accepted,
                ),
                UserInfo::new(
                    Sub::from("jora"),
                    Nickname::from("jora"),
                    "Jora",
                    Picture::parse("jora://picture").unwrap(),
                    Email::parse("jora@test.com").unwrap(),
                ),
            ),
            (
                ContactDto::new(
                    contact::Id("680d045617d7edcb069071d9".into()),
                    Sub::from("igor"),
                    contact::Status::Rejected,
                ),
                UserInfo::new(
                    Sub::from("igor"),
                    Nickname::from("igor"),
                    "Igor",
                    Picture::parse("igor://picture").unwrap(),
                    Email::parse("igor@test.com").unwrap(),
                ),
            ),
            (
                ContactDto::new(
                    contact::Id("680d045617d7edcb069071da".into()),
                    Sub::from("radu"),
                    contact::Status::Pending {
                        initiator: Sub::from("valera"),
                    },
                ),
                UserInfo::new(
                    Sub::from("radu"),
                    Nickname::from("radu"),
                    "Radu",
                    Picture::parse("radu://picture").unwrap(),
                    Email::parse("radu@test.com").unwrap(),
                ),
            ),
            (
                ContactDto::new(
                    contact::Id("680d045617d7edcb069071db".into()),
                    Sub::from("gicu"),
                    contact::Status::Pending {
                        initiator: Sub::from("gicu"),
                    },
                ),
                UserInfo::new(
                    Sub::from("gicu"),
                    Nickname::from("gicu"),
                    "Gicu",
                    Picture::parse("gicu://picture").unwrap(),
                    Email::parse("gicu@test.com").unwrap(),
                ),
            ),
            (
                ContactDto::new(
                    contact::Id("680d045617d7edcb069071dc".into()),
                    Sub::from("toha"),
                    contact::Status::Blocked {
                        initiator: Sub::from("valera"),
                    },
                ),
                UserInfo::new(
                    Sub::from("toha"),
                    Nickname::from("toha"),
                    "Toha",
                    Picture::parse("toha://picture").unwrap(),
                    Email::parse("toha@test.com").unwrap(),
                ),
            ),
            (
                ContactDto::new(
                    contact::Id("680d045617d7edcb069071dd".into()),
                    Sub::from("alex"),
                    contact::Status::Blocked {
                        initiator: Sub::from("alex"),
                    },
                ),
                UserInfo::new(
                    Sub::from("alex"),
                    Nickname::from("alex"),
                    "Alex",
                    Picture::parse("alex://picture").unwrap(),
                    Email::parse("alex@test.com").unwrap(),
                ),
            ),
        ];

        let actual = ContactInfos::new(&auth_user, &contact_infos)
            .render()
            .into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_pending_icon() {
        let expected = r#"<span class="text-blue-500"><i class="fa-solid fa-hourglass-half mr-2"></i>Pending action</span>"#;

        let actual = Icon::Pending.render().into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_accept_icon() {
        let id = contact::Id::random();
        let expected = format!(
            r##"<i class="fa-solid fa-check text-2xl text-green-600 cursor-pointer" hx-target="#ci-status-{}" hx-put="/api/contacts/{}/accept"></i>"##,
            &id, &id
        );

        let actual = Icon::Accept(&id).render().into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_reject_icon() {
        let id = contact::Id::random();
        let expected = format!(
            r##"<i class="fa-solid fa-xmark ml-3 text-2xl text-red-500 cursor-pointer" hx-target="#ci-status-{}" hx-put="/api/contacts/{}/reject"></i>"##,
            &id, &id
        );

        let actual = Icon::Reject(&id).render().into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_block_icon() {
        let id = contact::Id::random();
        let expected = format!(
            r##"<i class="fa-solid fa-ban ml-3 text-2xl cursor-pointer" hx-target="#ci-status-{}" hx-put="/api/contacts/{}/block"></i>"##,
            &id, &id
        );

        let actual = Icon::Block(&id).render().into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_unblock_icon() {
        let id = contact::Id::random();
        let expected = format!(
            r##"<i class="fa-solid fa-lock-open ml-3 text-green-600 text-xl cursor-pointer" hx-target="#ci-status-{}" hx-put="/api/contacts/{}/unblock"></i>"##,
            &id, &id
        );

        let actual = Icon::Unblock(&id).render().into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_rejected_icon() {
        let expected = r#"<span class="text-red-500"><i class="fa-solid fa-xmark mr-2"></i>Request rejected</span>"#;

        let actual = Icon::Rejected.render().into_string();

        assert_eq!(actual, expected);
    }
}
