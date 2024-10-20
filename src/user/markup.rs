use maud::{html, Markup, Render};

use super::model::UserInfo;

pub struct UserHeader<'a> {
    pub name: &'a str,
    pub picture: &'a str,
}

impl Render for UserHeader<'_> {
    fn render(&self) -> Markup {
        html! {
            header."flex justify-between items-center mb-4" {
                img."w-12 h-12 rounded-full mr-2"
                    src=(self.picture)
                    alt="User avatar" {}
                h2.text-2xl {(self.name)}
                a."bg-red-500 hover:bg-red-700 text-white font-bold py-2 px-4 rounded"
                    href="/logout" { "Logout" }
            }
        }
    }
}

pub struct UserSearch;

impl Render for UserSearch {
    fn render(&self) -> Markup {
        html! {
            input."mb-4 w-full px-3 py-2 border border-gray-300 rounded-md"
                type="search"
                name="nickname"
                placeholder="Who do you want to chat with? Type here..."
                hx-post="/api/users/search"
                hx-trigger="input changed delay:500ms"
                hx-target="#search-results" {}

            div id="search-results" class="relative" {}
        }
    }
}

pub fn search_result(users: &[UserInfo]) -> Markup {
    let search_result_class =
        "absolute w-full bg-white border border-gray-300 rounded-md shadow-lg cursor-pointer";
    html! {
        @if users.is_empty() {
            ul class=({search_result_class}) {
                li class="px-3 py-2" { "No users found" }
            }
        } @else {
            ul class=({search_result_class}) {
                @for user in users {
                    li class="px-3 py-2 hover:bg-gray-200" {
                        img class="w-6 h-6 rounded-full float-left"
                            src=(user.picture)
                            alt="User avatar" {}
                        strong.px-3 {(user.name)} (user.nickname)
                    }
                }
            }
        }
    }
}
