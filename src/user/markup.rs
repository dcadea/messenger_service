use std::fmt::Display;

use maud::{Markup, Render, html};

use crate::{
    auth,
    contact::{self, model::ContactDto},
    markup::IdExt,
    talk::markup::TALK_WINDOW_TARGET,
};

use super::{
    Sub,
    model::{OnlineStatus, UserInfo},
};

impl Display for Sub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct Header<'a>(pub &'a auth::User);

impl Render for Header<'_> {
    fn render(&self) -> Markup {
        html! {
            header #user-header ."flex items-center place-content-center mb-4" {
                img ."w-12 h-12 rounded-full mr-3"
                    src=(self.0.picture())
                    alt="User avatar" {}
                h2 .text-2xl { (self.0.name()) }
            }
        }
    }
}

pub struct Search;

const SEARCH_RESULTS_ID: &str = "search-results";
const SEARCH_RESULTS_TARGET: &str = "#search-results";

impl Render for Search {
    fn render(&self) -> Markup {
        let search_handler = format!(
            r"on keyup
                if the event's key is 'Escape'
                    set value of me to ''
                    remove children of {SEARCH_RESULTS_TARGET}"
        );

        html! {
            input ."mb-4 w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none"
                type="search"
                name="nickname"
                placeholder="Search users..."
                autocomplete="off"
                hx-post="/api/users/search"
                hx-trigger="input changed delay:500ms"
                hx-target=(SEARCH_RESULTS_TARGET)
                _=(search_handler) {}

            div #(SEARCH_RESULTS_ID) .relative {}
        }
    }
}

struct StartTalk<'a>(&'a Sub);

impl Render for StartTalk<'_> {
    fn render(&self) -> Markup {
        html! {
            form .float-right
                hx-post="/api/talks" // TODO: hx-get when chat exists
                hx-target=(TALK_WINDOW_TARGET)
                hx-ext="json-enc"
            {
                input type="hidden" name="kind" value="chat" {}
                input type="hidden" name="sub" value=(self.0) {}
                input ."px-2 py-1 text-white bg-blue-700 hover:bg-blue-800 font-medium rounded-lg text-xs focus:outline-none"
                    type="submit"
                    value="Start talk" {}
            }
        }
    }
}

struct AddContact<'a>(&'a Sub);

impl Render for AddContact<'_> {
    fn render(&self) -> Markup {
        html! {
            form .float-right hx-post="/api/contacts"
                hx-target="this"
                hx-swap="outerHTML"
            {
                input type="hidden" name="sub" value=(self.0) {}
                input ."px-2 py-1 text-white bg-green-700 hover:bg-green-800 font-medium rounded-lg text-xs focus:outline-none"
                    type="submit"
                    value="Add contact" {}
            }
        }
    }
}

pub struct SearchResult<'a> {
    contacts: &'a [ContactDto],
    users: &'a [UserInfo],
}

impl<'a> SearchResult<'a> {
    pub const fn new(contacts: &'a [ContactDto], users: &'a [UserInfo]) -> Self {
        Self { contacts, users }
    }
}

impl Render for SearchResult<'_> {
    fn render(&self) -> Markup {
        let search_result_class =
            "absolute w-full bg-white border border-gray-300 rounded-md shadow-lg";

        html! {
            ul .(search_result_class) {
                @if self.users.is_empty() {
                    li ."px-3 py-2" { "No users found" }
                } @else {
                    @for user in self.users {
                        li ."px-3 py-2" {
                            img ."w-6 h-6 rounded-full float-left"
                                src=(user.picture())
                                alt="User avatar" {}
                            strong .px-3 {(user.name())} (user.nickname())

                            @match self.contacts.iter().find(|c| user.sub().eq(c.recipient())) {
                                Some(c) => @match c.status() {
                                    contact::Status::Accepted => (StartTalk(user.sub())),
                                    _ => (c.status())
                                },
                                None => (AddContact(user.sub()))
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Render for contact::Status {
    fn render(&self) -> Markup {
        let status_class =
            "float-right px-2 py-1 text-white font-medium rounded-lg text-xs focus:outline-none";

        html! {
            @match self {
                Self::Pending{ .. } => span .(status_class) .bg-gray-400 { "Pending" },
                Self::Accepted => span .(status_class) .bg-green-700 { "Accepted" },
                Self::Rejected => span .(status_class) .bg-red-500 { "Rejected" },
                Self::Blocked { .. } => span .(status_class) .bg-red-700 { "Blocked" },
            }
        }
    }
}

impl crate::markup::IdExt for OnlineStatus {
    fn attr(&self) -> String {
        format!("os-{}", self.id())
    }

    fn target(&self) -> String {
        format!("#os-{}", self.id())
    }
}

impl Render for OnlineStatus {
    fn render(&self) -> Markup {
        html! {
            div sse-swap={"onlineStatusChange:"(self.id())}
                hx-target=(self.target())
                hx-swap="outerHTML"
            {
                (Icon::OnlineIndicator(self))
            }
        }
    }
}

pub enum Icon<'a> {
    OnlineIndicator(&'a OnlineStatus),
}

impl Render for Icon<'_> {
    fn render(&self) -> Markup {
        match self {
            Self::OnlineIndicator(os) => {
                let i_class = if os.online() {
                    "fa-solid"
                } else {
                    "fa-regular"
                };

                html! {
                    i #(os.attr()) .(i_class) ."fa-circle text-green-600 mr-2 text-sm" {}
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::user::{self};

    use super::*;

    #[test]
    fn should_render_header() {
        let expected = concat!(
            r#"<header class="flex items-center place-content-center mb-4" id="user-header">"#,
            r#"<img class="w-12 h-12 rounded-full mr-3" src="jora://url" alt="User avatar"></img>"#,
            r#"<h2 class="text-2xl">jora</h2>"#,
            "</header>"
        );

        let actual = Header(&auth::User::new(
            user::Sub("jora".into()),
            "jora_kardan",
            "jora",
            "jora://url",
        ))
        .render()
        .into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_search() {
        let expected = concat!(
            r##"<input class="mb-4 w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none" type="search" name="nickname" placeholder="Search users..." autocomplete="off" hx-post="/api/users/search" hx-trigger="input changed delay:500ms" hx-target="#search-results" "##,
            r##"_="on keyup
                if the event's key is 'Escape'
                    set value of me to ''
                    remove children of #search-results">"##,
            "</input>",
            r#"<div class="relative" id="search-results"></div>"#
        );

        let actual = Search.render().into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_start_talk() {
        let expected = html! {
            form class="float-right"
                hx-post="/api/talks"
                hx-target="#talk-window"
                hx-ext="json-enc"
            {
                input type="hidden" name="type" value="chat" {}
                input type="hidden" name="sub" value="valera" {}
                input class="px-2 py-1 text-white bg-blue-700 hover:bg-blue-800 font-medium rounded-lg text-xs focus:outline-none"
                    type="submit"
                    value="Start talk" {}
            }
        }.into_string();

        let actual = StartTalk(&user::Sub("valera".into()))
            .render()
            .into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_add_contact() {
        let expected = concat!(
            r#"<form class="float-right" hx-post="/api/contacts" hx-target="this" hx-swap="outerHTML">"#,
            r#"<input type="hidden" name="sub" value="radu"></input>"#,
            r#"<input class="px-2 py-1 text-white bg-green-700 hover:bg-green-800 font-medium rounded-lg text-xs focus:outline-none" type="submit" value="Add contact"></input>"#,
            "</form>"
        );

        let actual = AddContact(&user::Sub("radu".into())).render().into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_empty_search_result() {
        let expected = html! {
            ul class="absolute w-full bg-white border border-gray-300 rounded-md shadow-lg" {
                li class="px-3 py-2" { "No users found" }
            }
        }
        .into_string();

        let actual = SearchResult::new(&[], &[]).render().into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_search_result() {
        let expected = html! {
            ul class="absolute w-full bg-white border border-gray-300 rounded-md shadow-lg" {
                li class="px-3 py-2" {
                    img class="w-6 h-6 rounded-full float-left" src="jora" alt="User avatar" {}
                    strong class="px-3" {"Jora"} "jora"
                    form class="float-right" hx-post="/api/talks" hx-target="#talk-window" hx-ext="json-enc" {
                        input type="hidden" name="type" value="chat" {}
                        input type="hidden" name="sub" value="jora" {}
                        input class="px-2 py-1 text-white bg-blue-700 hover:bg-blue-800 font-medium rounded-lg text-xs focus:outline-none"
                            type="submit"
                            value="Start talk" {}
                    }
                }
                li class="px-3 py-2" {
                    img class="w-6 h-6 rounded-full float-left" src="radu" alt="User avatar" {}
                    strong class="px-3" {"Radu"} "radu"
                    form class="float-right" hx-post="/api/contacts" hx-target="this" hx-swap="outerHTML" {
                        input type="hidden" name="sub" value="radu" {}
                        input class="px-2 py-1 text-white bg-green-700 hover:bg-green-800 font-medium rounded-lg text-xs focus:outline-none"
                            type="submit"
                            value="Add contact" {}
                    }
                }
                li class="px-3 py-2" {
                    img class="w-6 h-6 rounded-full float-left" src="igor" alt="User avatar" {}
                    strong class="px-3" {"Igor"} "igor"
                    span class="float-right px-2 py-1 text-white font-medium rounded-lg text-xs focus:outline-none bg-red-500" {
                        "Rejected"
                    }
                }
            }
        }.into_string();

        let contacts = [
            ContactDto::new(
                contact::Id::random(),
                user::Sub("jora".into()),
                contact::Status::Accepted,
            ),
            ContactDto::new(
                contact::Id::random(),
                user::Sub("igor".into()),
                contact::Status::Rejected,
            ),
        ];

        let user_infos = [
            UserInfo::new(user::Sub("jora".into()), "jora", "Jora", "jora", "jora"),
            UserInfo::new(user::Sub("radu".into()), "radu", "Radu", "radu", "radu"),
            UserInfo::new(user::Sub("igor".into()), "igor", "Igor", "igor", "igor"),
        ];
        let actual = SearchResult::new(&contacts, &user_infos)
            .render()
            .into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_status_pending() {
        let expected = r#"<span class="float-right px-2 py-1 text-white font-medium rounded-lg text-xs focus:outline-none bg-gray-400">Pending</span>"#;

        let actuat = contact::Status::Pending {
            initiator: user::Sub("valera".into()),
        }
        .render()
        .into_string();

        assert_eq!(actuat, expected);
    }

    #[test]
    fn should_render_status_accepted() {
        let expected = r#"<span class="float-right px-2 py-1 text-white font-medium rounded-lg text-xs focus:outline-none bg-green-700">Accepted</span>"#;

        let actuat = contact::Status::Accepted.render().into_string();

        assert_eq!(actuat, expected);
    }

    #[test]
    fn should_render_status_rejected() {
        let expected = r#"<span class="float-right px-2 py-1 text-white font-medium rounded-lg text-xs focus:outline-none bg-red-500">Rejected</span>"#;

        let actuat = contact::Status::Rejected.render().into_string();

        assert_eq!(actuat, expected);
    }

    #[test]
    fn should_render_status_blocked() {
        let expected = r#"<span class="float-right px-2 py-1 text-white font-medium rounded-lg text-xs focus:outline-none bg-red-700">Blocked</span>"#;

        let actuat = contact::Status::Blocked {
            initiator: user::Sub("valera".into()),
        }
        .render()
        .into_string();

        assert_eq!(actuat, expected);
    }

    #[test]
    fn should_render_online_status() {
        let expected = concat!(
            r##"<div sse-swap="onlineStatusChange:igor" hx-target="#os-igor" hx-swap="outerHTML">"##,
            r#"<i class="fa-solid fa-circle text-green-600 mr-2 text-sm" id="os-igor"></i>"#,
            "</div>"
        );

        let actual = OnlineStatus::new(user::Sub("google|igor".into()), true)
            .render()
            .into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_online_indicator_icon_when_online_status_is_true() {
        let expected =
            r#"<i class="fa-solid fa-circle text-green-600 mr-2 text-sm" id="os-igor"></i>"#;

        let actual =
            Icon::OnlineIndicator(&OnlineStatus::new(user::Sub("google|igor".into()), true))
                .render()
                .into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_online_indicator_icon_when_online_status_is_false() {
        let expected =
            r#"<i class="fa-regular fa-circle text-green-600 mr-2 text-sm" id="os-jora"></i>"#;

        let actual =
            Icon::OnlineIndicator(&OnlineStatus::new(user::Sub("auth0|jora".into()), false))
                .render()
                .into_string();

        assert_eq!(actual, expected);
    }
}
