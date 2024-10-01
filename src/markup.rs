use maud::{html, Markup, Render, DOCTYPE};

use crate::user::markup::UserHeader;

struct Script<'a>(&'a str);

impl Render for Script<'_> {
    fn render(&self) -> Markup {
        html! {
            script src=(self.0) {}
        }
    }
}

struct Css<'a>(&'a str);

impl Render for Css<'_> {
    fn render(&self) -> Markup {
        html! {
            link rel="stylesheet" href=(self.0) {}
        }
    }
}

pub(super) async fn root() -> Markup {
    base(html! {
        #chat-window ."flex flex-col h-full"
            hx-get="/api/chats"
            hx-trigger="load"
            hx-swap="beforeend"
            hx-ext="ws"
            ws-connect="/ws"
        {
            (UserHeader{
                name: "Dmitrii Cadea",
                picture: "https://avatars.githubusercontent.com/u/10639696?v=4"
            })
        }
    })
}

pub(crate) fn base(content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                meta charset="utf-8" {}
                title { "AWG Messenger" }
                (Script("https://unpkg.com/htmx.org@2.0.2"))
                (Script("https://unpkg.com/htmx.org@2.0.2/dist/ext/ws.js"))
                (Script("https://cdn.tailwindcss.com"))
                (Css("https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.6.0/css/all.min.css"))
            }
            body."h-screen bg-black flex items-center justify-center" {
                ."max-w-lg h-3/5 md:h-4/5 md:w-4/5 bg-white rounded-2xl p-6" {
                    (content)
                }
            }
        }
    }
}
