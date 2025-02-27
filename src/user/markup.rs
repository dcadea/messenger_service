use std::collections::HashSet;

use maud::{Markup, Render, html};
use messenger_service::markup::Id;

use crate::chat::markup::CHAT_WINDOW_TARGET;

use super::{
    Sub,
    model::{OnlineStatus, UserInfo},
};

pub struct Header<'a>(pub &'a UserInfo);

impl Render for Header<'_> {
    fn render(&self) -> Markup {
        html! {
            header #user-header ."flex justify-between items-center mb-4" {
                img ."w-12 h-12 rounded-full mr-2"
                    src=(self.0.picture)
                    alt="User avatar" {}
                h2 .text-2xl {(self.0.name)}
                a ."bg-red-500 hover:bg-red-700 text-white font-bold py-2 px-4 rounded"
                    href="/logout" { "Logout" }
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
            r#"on keyup
                if the event's key is 'Escape'
                    set value of me to ''
                    remove children of {}
            "#,
            SEARCH_RESULTS_TARGET
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

struct AddFriend<'a>(&'a Sub);

impl Render for AddFriend<'_> {
    fn render(&self) -> Markup {
        html! {
            form .float-right
                hx-post="/api/chats"
                hx-target=(CHAT_WINDOW_TARGET)
            {
                input type="hidden" name="sub" value=(self.0) {}
                input ."px-2 py-1 text-white bg-green-700 hover:bg-green-800 font-medium rounded-lg text-xs focus:outline-none"
                    type="submit"
                    value="Add friend" {}
            }
        }
    }
}

pub struct SearchResult<'a> {
    friends: &'a HashSet<Sub>,
    users: &'a [UserInfo],
}

impl<'a> SearchResult<'a> {
    pub fn new(friends: &'a HashSet<Sub>, users: &'a [UserInfo]) -> Self {
        Self { friends, users }
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
                                src=(user.picture)
                                alt="User avatar" {}
                            strong .px-3 {(user.name)} (user.nickname)

                            @if !self.friends.contains(&user.sub) {
                                (AddFriend(&user.sub))
                            }
                        }
                    }
                }
            }
        }
    }
}

impl messenger_service::markup::Id for OnlineStatus {
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
                let i_class = match os.online {
                    true => "fa-solid",
                    false => "fa-regular",
                };

                html! {
                    i #(os.attr()) .(i_class) ."fa-circle text-green-600 mr-2" {}
                }
            }
        }
    }
}
