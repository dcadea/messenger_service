use std::fmt::Display;

use maud::{Markup, Render, html};

use crate::{auth, contact, contact::Status, user::model::UserInfo};

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
    pub fn new(auth_user: &'a auth::User, contact_infos: &'a [(ContactDto, UserInfo)]) -> Self {
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
        let auth_sub = &self.auth_user.sub();

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

                        div #{"ci-status-" (c.id())}
                            ."grow text-right"
                            .text-blue-500[c.is_pending()]
                            .text-red-500[c.is_rejected()]
                        {
                            @match c.status() {
                                Status::Pending { initiator } => {
                                    @if initiator.eq(auth_sub) {
                                        (Icon::Pending)
                                    } @else {
                                        (Icon::Accept(c.id()))
                                        (Icon::Reject(c.id()))
                                    }
                                },
                                Status::Accepted => (Icon::Block(c.id())),
                                Status::Rejected => (Icon::Rejected),
                                Status::Blocked { initiator } => {
                                    @if initiator.eq(auth_sub) {
                                        "Blocked"
                                        (Icon::Unblock(c.id()))
                                    } @else {
                                        "Blocked you"
                                    }
                                },
                            }
                        }
                    }
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
        html! {
            @match self {
                Self::Pending => {
                    i ."fa-solid fa-hourglass-half mr-2" {}
                    "Pending action"
                },
                Self::Accept(id) => {
                    i ."fa-solid fa-check text-2xl text-green-500 cursor-pointer"
                        hx-swap="none" // TODO: remove icons after accept
                        hx-put={"/api/contacts/" (id) "/accept"} {}
                },
                Self::Reject(id) => {
                    i ."fa-solid fa-xmark ml-3 text-2xl text-red-500 cursor-pointer"
                        hx-swap="none" // TODO: remove icons after reject
                        hx-put={"/api/contacts/" (id) "/reject"} {}
                },
                Self::Block(id) => {
                    i ."fa-solid fa-ban ml-3 text-2xl cursor-pointer"
                        hx-swap="none" // TODO: remove icon after block
                        hx-put={"/api/contacts/" (id) "/block"} {}
                },
                Self::Unblock(id) => {
                    i ."fa-solid fa-lock-open ml-3 text-green-500 text-xl cursor-pointer"
                        hx-swap="none" // TODO: remove icon after unblock
                        hx-put={"/api/contacts/" (id) "/unblock"} {}
                },
                Self::Rejected => {
                    i ."fa-solid fa-xmark mr-2" {}
                    "Request rejected"
                },
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::user;

    use super::*;

    #[test]
    fn should_render_contact_infos() {
        let expected = concat!(
            r#"<header class="text-center mb-4">"#,
            r#"<h2 class="text-2xl">Contacts</h2>"#,
            "</header>",
            r#"<ul class="flex flex-col space-y-2">"#,
            r#"<li class="px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center">"#,
            r#"<img class="w-9 h-9 rounded-full float-left mr-2" src="jora://picture" alt="User avatar"></img>Jora"#,
            r#"<div class="grow text-right" id="ci-status-680d045617d7edcb069071d8">"#,
            r#"<i class="fa-solid fa-ban ml-3 text-2xl cursor-pointer" hx-swap="none" hx-put="/api/contacts/680d045617d7edcb069071d8/block"></i>"#,
            "</div>",
            "</li>",
            r#"<li class="px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center">"#,
            r#"<img class="w-9 h-9 rounded-full float-left mr-2" src="igor://picture" alt="User avatar"></img>Igor"#,
            r#"<div class="grow text-right text-red-500" id="ci-status-680d045617d7edcb069071d9">"#,
            r#"<i class="fa-solid fa-xmark mr-2"></i>Request rejected"#,
            "</div>",
            "</li>",
            r#"<li class="px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center">"#,
            r#"<img class="w-9 h-9 rounded-full float-left mr-2" src="radu://picture" alt="User avatar"></img>Radu"#,
            r#"<div class="grow text-right text-blue-500" id="ci-status-680d045617d7edcb069071da">"#,
            r#"<i class="fa-solid fa-hourglass-half mr-2"></i>Pending action"#,
            "</div>",
            "</li>",
            r#"<li class="px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center">"#,
            r#"<img class="w-9 h-9 rounded-full float-left mr-2" src="gicu://picture" alt="User avatar"></img>Gicu"#,
            r#"<div class="grow text-right text-blue-500" id="ci-status-680d045617d7edcb069071db">"#,
            r#"<i class="fa-solid fa-check text-2xl text-green-500 cursor-pointer" hx-swap="none" hx-put="/api/contacts/680d045617d7edcb069071db/accept"></i>"#,
            r#"<i class="fa-solid fa-xmark ml-3 text-2xl text-red-500 cursor-pointer" hx-swap="none" hx-put="/api/contacts/680d045617d7edcb069071db/reject"></i>"#,
            "</div>",
            "</li>",
            r#"<li class="px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center">"#,
            r#"<img class="w-9 h-9 rounded-full float-left mr-2" src="toha://picture" alt="User avatar"></img>Toha"#,
            r#"<div class="grow text-right" id="ci-status-680d045617d7edcb069071dc">Blocked"#,
            r#"<i class="fa-solid fa-lock-open ml-3 text-green-500 text-xl cursor-pointer" hx-swap="none" hx-put="/api/contacts/680d045617d7edcb069071dc/unblock"></i>"#,
            "</div>",
            "</li>",
            r#"<li class="px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center">"#,
            r#"<img class="w-9 h-9 rounded-full float-left mr-2" src="alex://picture" alt="User avatar"></img>Alex"#,
            r#"<div class="grow text-right" id="ci-status-680d045617d7edcb069071dd">Blocked you</div>"#,
            "</li>",
            "</ul>"
        );

        let auth_user = auth::User::new(
            user::Sub("valera".into()),
            "valera",
            "Valera",
            "valera://picture",
        );

        let contact_infos = [
            (
                ContactDto::new(
                    contact::Id("680d045617d7edcb069071d8".into()),
                    user::Sub("jora".into()),
                    contact::Status::Accepted,
                ),
                UserInfo::new(
                    user::Sub("jora".into()),
                    "jora",
                    "Jora",
                    "jora://picture",
                    "jora@test.com",
                ),
            ),
            (
                ContactDto::new(
                    contact::Id("680d045617d7edcb069071d9".into()),
                    user::Sub("igor".into()),
                    contact::Status::Rejected,
                ),
                UserInfo::new(
                    user::Sub("igor".into()),
                    "igor",
                    "Igor",
                    "igor://picture",
                    "igor@test.com",
                ),
            ),
            (
                ContactDto::new(
                    contact::Id("680d045617d7edcb069071da".into()),
                    user::Sub("radu".into()),
                    contact::Status::Pending {
                        initiator: user::Sub("valera".into()),
                    },
                ),
                UserInfo::new(
                    user::Sub("radu".into()),
                    "radu",
                    "Radu",
                    "radu://picture",
                    "radu@test.com",
                ),
            ),
            (
                ContactDto::new(
                    contact::Id("680d045617d7edcb069071db".into()),
                    user::Sub("gicu".into()),
                    contact::Status::Pending {
                        initiator: user::Sub("gicu".into()),
                    },
                ),
                UserInfo::new(
                    user::Sub("gicu".into()),
                    "gicu",
                    "Gicu",
                    "gicu://picture",
                    "gicu@test.com",
                ),
            ),
            (
                ContactDto::new(
                    contact::Id("680d045617d7edcb069071dc".into()),
                    user::Sub("toha".into()),
                    contact::Status::Blocked {
                        initiator: user::Sub("valera".into()),
                    },
                ),
                UserInfo::new(
                    user::Sub("toha".into()),
                    "toha",
                    "Toha",
                    "toha://picture",
                    "toha@test.com",
                ),
            ),
            (
                ContactDto::new(
                    contact::Id("680d045617d7edcb069071dd".into()),
                    user::Sub("alex".into()),
                    contact::Status::Blocked {
                        initiator: user::Sub("alex".into()),
                    },
                ),
                UserInfo::new(
                    user::Sub("alex".into()),
                    "alex",
                    "Alex",
                    "alex://picture",
                    "alex@test.com",
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
        let expected = r#"<i class="fa-solid fa-hourglass-half mr-2"></i>Pending action"#;

        let actual = Icon::Pending.render().into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_accept_icon() {
        let id = contact::Id::random();
        let expected = format!(
            r#"<i class="fa-solid fa-check text-2xl text-green-500 cursor-pointer" hx-swap="none" hx-put="/api/contacts/{}/accept"></i>"#,
            &id
        );

        let actual = Icon::Accept(&id).render().into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_reject_icon() {
        let id = contact::Id::random();
        let expected = format!(
            r#"<i class="fa-solid fa-xmark ml-3 text-2xl text-red-500 cursor-pointer" hx-swap="none" hx-put="/api/contacts/{}/reject"></i>"#,
            &id
        );

        let actual = Icon::Reject(&id).render().into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_block_icon() {
        let id = contact::Id::random();
        let expected = format!(
            r#"<i class="fa-solid fa-ban ml-3 text-2xl cursor-pointer" hx-swap="none" hx-put="/api/contacts/{}/block"></i>"#,
            &id
        );

        let actual = Icon::Block(&id).render().into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_unblock_icon() {
        let id = contact::Id::random();
        let expected = format!(
            r#"<i class="fa-solid fa-lock-open ml-3 text-green-500 text-xl cursor-pointer" hx-swap="none" hx-put="/api/contacts/{}/unblock"></i>"#,
            &id
        );

        let actual = Icon::Unblock(&id).render().into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_rejected_icon() {
        let expected = r#"<i class="fa-solid fa-xmark mr-2"></i>Request rejected"#;

        let actual = Icon::Rejected.render().into_string();

        assert_eq!(actual, expected);
    }
}
