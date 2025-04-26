use std::convert::Infallible;

use axum::{
    body::Body,
    response::{IntoResponse, IntoResponseParts, Response, ResponseParts},
};
use maud::{DOCTYPE, Markup, PreEscaped, Render, html};
use reqwest::header::CONTENT_LENGTH;

pub const EMPTY: PreEscaped<&'static str> = PreEscaped("");

pub trait IdExt {
    fn attr(&self) -> String;
    fn target(&self) -> String;
}

struct Head<'a>(&'a str);

impl Render for Head<'_> {
    fn render(&self) -> Markup {
        html! {
            head {
                meta charset="utf-8" {}
                title { (self.0) }
                script src="https://unpkg.com/htmx.org@2.0.4" {}
                script src="https://unpkg.com/htmx-ext-ws@2.0.3/ws.js" {}
                script src="https://unpkg.com/htmx-ext-sse@2.2.3/sse.js" {}
                script src="https://unpkg.com/hyperscript.org@0.9.13" {}

                script src="https://unpkg.com/@tailwindcss/browser@4" {}
                script src="/static/scripts.js" {}

                link rel="stylesheet" href="/static/styles.css" {}
                link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.6.0/css/all.min.css" {}

                meta name="htmx-config" content=r#"{"responseHandling": [{"code":".*", "swap": true}]}"# {}
            }
        }
    }
}

struct Screen<'a>(&'a Markup);

impl Render for Screen<'_> {
    fn render(&self) -> Markup {
        html! {
            #screen
                ."max-w-lg h-3/5 md:w-4/5 w-full"
                ."bg-white rounded-2xl"
                ."overflow-hidden"
            {
                (self.0)
            }
        }
    }
}

fn base(w: &Wrappable) -> Markup {
    html! {
        (DOCTYPE)
        html {
            (Head("AWG Messenger"))

            body ."h-screen bg-black flex items-center justify-center" {
                (Screen(&w.content))
            }
        }
    }
}

#[derive(Clone)]
pub struct Wrappable {
    content: Markup,
}

impl Wrappable {
    pub fn new(content: impl Render) -> Self {
        Self {
            content: content.render(),
        }
    }
}

impl IntoResponseParts for Wrappable {
    type Error = Infallible;

    fn into_response_parts(
        self,
        mut res: ResponseParts,
    ) -> core::result::Result<ResponseParts, Self::Error> {
        res.extensions_mut().insert(self);
        Ok(res)
    }
}

impl IntoResponse for Wrappable {
    fn into_response(self) -> axum::response::Response {
        (self, ()).into_response()
    }
}

pub fn wrap_in_base(mut resp: Response) -> impl IntoResponse {
    if let Some(w) = resp.extensions_mut().remove::<Wrappable>() {
        resp.headers_mut().remove(CONTENT_LENGTH);
        *resp.body_mut() = Body::new(base(&w).into_string());
        return resp;
    }

    resp
}

#[derive(PartialEq)]
pub enum TabControls {
    Chats,
    Groups,
    Contacts,
    Settings,
}

impl Render for TabControls {
    fn render(&self) -> Markup {
        html! {
            div ."flex flex-row text-2xl" role="tablist" {
                (TabControlItem::Chats(self))
                (TabControlItem::Groups(self))
                (TabControlItem::Contacts(self))
                (TabControlItem::Settings(self))
            }
        }
    }
}

enum TabControlItem<'a> {
    Chats(&'a TabControls),
    Groups(&'a TabControls),
    Contacts(&'a TabControls),
    Settings(&'a TabControls),
}

impl Render for TabControlItem<'_> {
    fn render(&self) -> Markup {
        let (path, active, i_class) = match self {
            TabControlItem::Chats(at) => (
                "/tabs/chats",
                TabControls::Chats.eq(at),
                "fa-regular fa-message",
            ),
            TabControlItem::Groups(at) => (
                "/tabs/groups",
                TabControls::Groups.eq(at),
                "fa-solid fa-people-group",
            ),
            TabControlItem::Contacts(at) => (
                "/tabs/contacts",
                TabControls::Contacts.eq(at),
                "fa-regular fa-address-book",
            ),
            TabControlItem::Settings(at) => (
                "/tabs/settings",
                TabControls::Settings.eq(at),
                "fa-solid fa-gears",
            ),
        };

        html! {
            button ."basis-64 py-4 hover:bg-gray-300 cursor-pointer"
                hx-get=(path)
                role="tab"
                .bg-gray-100[active]
                aria-selected=(active)
                aria-controls="tab-content"
            {
                i .(i_class) {}
            }
        }
    }
}

pub struct Tab {
    controls: TabControls,
    content: Markup,
}

impl Tab {
    pub fn new(controls: TabControls, content: impl Render) -> Self {
        Self {
            controls,
            content: content.render(),
        }
    }
}

impl Render for Tab {
    fn render(&self) -> Markup {
        html! {
            #tab-content ."flex-1 px-6 pt-6 relative overflow-auto" role="tabpanel" {
                #errors {}

                (self.content)
            }

            (self.controls)
        }
    }
}

pub struct Tabs;

impl Render for Tabs {
    fn render(&self) -> Markup {
        html! {
            #tabs
                ."flex flex-col w-full h-full"
                hx-get="/tabs/chats"
                hx-trigger="load"
                hx-target="#tabs"
                hx-swap="innerHTML"
                hx-ext="sse"
                sse-connect="/sse" {}
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_render_head() {
        let expected = concat!(
            "<head>",
            r#"<meta charset="utf-8"></meta>"#,
            "<title>AWG Messenger</title>",
            r#"<script src="https://unpkg.com/htmx.org@2.0.4"></script>"#,
            r#"<script src="https://unpkg.com/htmx-ext-ws@2.0.3/ws.js"></script>"#,
            r#"<script src="https://unpkg.com/htmx-ext-sse@2.2.3/sse.js"></script>"#,
            r#"<script src="https://unpkg.com/hyperscript.org@0.9.13"></script>"#,
            r#"<script src="https://unpkg.com/@tailwindcss/browser@4"></script>"#,
            r#"<script src="/static/scripts.js"></script>"#,
            r#"<link rel="stylesheet" href="/static/styles.css"></link>"#,
            r#"<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.6.0/css/all.min.css"></link>"#,
            r#"<meta name="htmx-config" content="{&quot;responseHandling&quot;: [{&quot;code&quot;:&quot;.*&quot;, &quot;swap&quot;: true}]}"></meta>"#,
            "</head>"
        );

        let actual = Head("AWG Messenger").render().into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_screen() {
        let expected = concat!(
            r#"<div class="max-w-lg h-3/5 md:w-4/5 w-full bg-white rounded-2xl overflow-hidden" id="screen">"#,
            r#"<div class="flex flex-col h-screen">"#,
            r#"<header class="bg-gray-800 text-white p-4">"#,
            r#"<h1 class="text-2xl font-bold">AWG Messenger</h1>"#,
            "</header>",
            r#"<main class="flex-1 overflow-y-auto">"#,
            r#"<div class="p-4"><p>Welcome to AWG Messenger!</p></div>"#,
            "</main>",
            r#"<footer class="bg-gray-800 text-white p-4"><p>2023 AWG Messenger</p></footer>"#,
            "</div>",
            "</div>"
        );

        let actual = Screen(&html! {
            ."flex flex-col h-screen" {
                header ."bg-gray-800 text-white p-4" {
                    h1 ."text-2xl font-bold" { "AWG Messenger" }
                }
                main ."flex-1 overflow-y-auto" {
                    ."p-4" {
                        p { "Welcome to AWG Messenger!" }
                    }
                }
                footer ."bg-gray-800 text-white p-4" {
                    p { "2023 AWG Messenger" }
                }
            }
        })
        .render()
        .into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_base() {
        let expected = concat!(
            "<!DOCTYPE html>",
            "<html>",
            "<head>",
            r#"<meta charset="utf-8"></meta>"#,
            "<title>AWG Messenger</title>",
            r#"<script src="https://unpkg.com/htmx.org@2.0.4"></script>"#,
            r#"<script src="https://unpkg.com/htmx-ext-ws@2.0.3/ws.js"></script>"#,
            r#"<script src="https://unpkg.com/htmx-ext-sse@2.2.3/sse.js"></script>"#,
            r#"<script src="https://unpkg.com/hyperscript.org@0.9.13"></script>"#,
            r#"<script src="https://unpkg.com/@tailwindcss/browser@4"></script>"#,
            r#"<script src="/static/scripts.js"></script>"#,
            r#"<link rel="stylesheet" href="/static/styles.css"></link>"#,
            r#"<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.6.0/css/all.min.css"></link>"#,
            r#"<meta name="htmx-config" content="{&quot;responseHandling&quot;: [{&quot;code&quot;:&quot;.*&quot;, &quot;swap&quot;: true}]}"></meta>"#,
            "</head>",
            r#"<body class="h-screen bg-black flex items-center justify-center">"#,
            r#"<div class="max-w-lg h-3/5 md:w-4/5 w-full bg-white rounded-2xl overflow-hidden" id="screen">"#,
            r#"<div class="flex flex-col h-screen">"#,
            r#"<header class="bg-gray-800 text-white p-4">"#,
            r#"<h1 class="text-2xl font-bold">AWG Messenger</h1>"#,
            "</header>",
            r#"<main class="flex-1 overflow-y-auto">"#,
            r#"<div class="p-4"><p>Welcome to AWG Messenger!</p></div>"#,
            "</main>",
            r#"<footer class="bg-gray-800 text-white p-4"><p>2023 AWG Messenger</p></footer>"#,
            "</div>",
            "</div>",
            "</body>",
            "</html>"
        );

        let actual = base(&Wrappable::new(html! {
            ."flex flex-col h-screen" {
                header ."bg-gray-800 text-white p-4" {
                    h1 ."text-2xl font-bold" { "AWG Messenger" }
                }
                main ."flex-1 overflow-y-auto" {
                    ."p-4" {
                        p { "Welcome to AWG Messenger!" }
                    }
                }
                footer ."bg-gray-800 text-white p-4" {
                    p { "2023 AWG Messenger" }
                }
            }
        }))
        .render()
        .into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_tab_controls_with_active_chats() {
        let expected = concat!(
            r#"<div class="flex flex-row text-2xl" role="tablist">"#,
            r#"<button class="basis-64 py-4 hover:bg-gray-300 cursor-pointer bg-gray-100" hx-get="/tabs/chats" role="tab" aria-selected="true" aria-controls="tab-content">"#,
            r#"<i class="fa-regular fa-message"></i>"#,
            "</button>",
            r#"<button class="basis-64 py-4 hover:bg-gray-300 cursor-pointer" hx-get="/tabs/groups" role="tab" aria-selected="false" aria-controls="tab-content">"#,
            r#"<i class="fa-solid fa-people-group"></i>"#,
            "</button>",
            r#"<button class="basis-64 py-4 hover:bg-gray-300 cursor-pointer" hx-get="/tabs/contacts" role="tab" aria-selected="false" aria-controls="tab-content">"#,
            r#"<i class="fa-regular fa-address-book"></i>"#,
            "</button>",
            r#"<button class="basis-64 py-4 hover:bg-gray-300 cursor-pointer" hx-get="/tabs/settings" role="tab" aria-selected="false" aria-controls="tab-content">"#,
            r#"<i class="fa-solid fa-gears"></i>"#,
            "</button>",
            "</div>"
        );

        let actual = TabControls::Chats.render().into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_tab_controls_with_active_groups() {
        let expected = concat!(
            r#"<div class="flex flex-row text-2xl" role="tablist">"#,
            r#"<button class="basis-64 py-4 hover:bg-gray-300 cursor-pointer" hx-get="/tabs/chats" role="tab" aria-selected="false" aria-controls="tab-content">"#,
            r#"<i class="fa-regular fa-message"></i>"#,
            "</button>",
            r#"<button class="basis-64 py-4 hover:bg-gray-300 cursor-pointer bg-gray-100" hx-get="/tabs/groups" role="tab" aria-selected="true" aria-controls="tab-content">"#,
            r#"<i class="fa-solid fa-people-group"></i>"#,
            "</button>",
            r#"<button class="basis-64 py-4 hover:bg-gray-300 cursor-pointer" hx-get="/tabs/contacts" role="tab" aria-selected="false" aria-controls="tab-content">"#,
            r#"<i class="fa-regular fa-address-book"></i>"#,
            "</button>",
            r#"<button class="basis-64 py-4 hover:bg-gray-300 cursor-pointer" hx-get="/tabs/settings" role="tab" aria-selected="false" aria-controls="tab-content">"#,
            r#"<i class="fa-solid fa-gears"></i>"#,
            "</button>",
            "</div>"
        );

        let actual = TabControls::Groups.render().into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_tab_controls_with_active_contacts() {
        let expected = concat!(
            r#"<div class="flex flex-row text-2xl" role="tablist">"#,
            r#"<button class="basis-64 py-4 hover:bg-gray-300 cursor-pointer" hx-get="/tabs/chats" role="tab" aria-selected="false" aria-controls="tab-content">"#,
            r#"<i class="fa-regular fa-message"></i>"#,
            "</button>",
            r#"<button class="basis-64 py-4 hover:bg-gray-300 cursor-pointer" hx-get="/tabs/groups" role="tab" aria-selected="false" aria-controls="tab-content">"#,
            r#"<i class="fa-solid fa-people-group"></i>"#,
            "</button>",
            r#"<button class="basis-64 py-4 hover:bg-gray-300 cursor-pointer bg-gray-100" hx-get="/tabs/contacts" role="tab" aria-selected="true" aria-controls="tab-content">"#,
            r#"<i class="fa-regular fa-address-book"></i>"#,
            "</button>",
            r#"<button class="basis-64 py-4 hover:bg-gray-300 cursor-pointer" hx-get="/tabs/settings" role="tab" aria-selected="false" aria-controls="tab-content">"#,
            r#"<i class="fa-solid fa-gears"></i>"#,
            "</button>",
            "</div>"
        );

        let actual = TabControls::Contacts.render().into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_tab_controls_with_active_settings() {
        let expected = concat!(
            r#"<div class="flex flex-row text-2xl" role="tablist">"#,
            r#"<button class="basis-64 py-4 hover:bg-gray-300 cursor-pointer" hx-get="/tabs/chats" role="tab" aria-selected="false" aria-controls="tab-content">"#,
            r#"<i class="fa-regular fa-message"></i>"#,
            "</button>",
            r#"<button class="basis-64 py-4 hover:bg-gray-300 cursor-pointer" hx-get="/tabs/groups" role="tab" aria-selected="false" aria-controls="tab-content">"#,
            r#"<i class="fa-solid fa-people-group"></i>"#,
            "</button>",
            r#"<button class="basis-64 py-4 hover:bg-gray-300 cursor-pointer" hx-get="/tabs/contacts" role="tab" aria-selected="false" aria-controls="tab-content">"#,
            r#"<i class="fa-regular fa-address-book"></i>"#,
            "</button>",
            r#"<button class="basis-64 py-4 hover:bg-gray-300 cursor-pointer bg-gray-100" hx-get="/tabs/settings" role="tab" aria-selected="true" aria-controls="tab-content">"#,
            r#"<i class="fa-solid fa-gears"></i>"#,
            "</button>",
            "</div>"
        );

        let actual = TabControls::Settings.render().into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_tab() {
        let expected = concat!(
            r#"<div class="flex-1 px-6 pt-6 relative overflow-auto" id="tab-content" role="tabpanel">"#,
            r#"<div id="errors"></div>"#,
            r#"<div class="flex flex-col h-screen">"#,
            r#"<header class="bg-gray-800 text-white p-4">"#,
            r#"<h1 class="text-2xl font-bold">AWG Messenger</h1>"#,
            "</header>",
            r#"<main class="flex-1 overflow-y-auto">"#,
            r#"<div class="p-4"><p>Welcome to AWG Messenger!</p></div>"#,
            "</main>",
            r#"<footer class="bg-gray-800 text-white p-4"><p>2023 AWG Messenger</p></footer>"#,
            "</div>",
            "</div>",
            r#"<div class="flex flex-row text-2xl" role="tablist">"#,
            r#"<button class="basis-64 py-4 hover:bg-gray-300 cursor-pointer bg-gray-100" hx-get="/tabs/chats" role="tab" aria-selected="true" aria-controls="tab-content">"#,
            r#"<i class="fa-regular fa-message"></i>"#,
            "</button>",
            r#"<button class="basis-64 py-4 hover:bg-gray-300 cursor-pointer" hx-get="/tabs/groups" role="tab" aria-selected="false" aria-controls="tab-content">"#,
            r#"<i class="fa-solid fa-people-group"></i>"#,
            "</button>",
            r#"<button class="basis-64 py-4 hover:bg-gray-300 cursor-pointer" hx-get="/tabs/contacts" role="tab" aria-selected="false" aria-controls="tab-content">"#,
            r#"<i class="fa-regular fa-address-book"></i>"#,
            "</button>",
            r#"<button class="basis-64 py-4 hover:bg-gray-300 cursor-pointer" hx-get="/tabs/settings" role="tab" aria-selected="false" aria-controls="tab-content">"#,
            r#"<i class="fa-solid fa-gears"></i>"#,
            "</button>",
            "</div>"
        );

        let actual = Tab::new(
            TabControls::Chats,
            html! {
                ."flex flex-col h-screen" {
                    header ."bg-gray-800 text-white p-4" {
                        h1 ."text-2xl font-bold" { "AWG Messenger" }
                    }
                    main ."flex-1 overflow-y-auto" {
                        ."p-4" {
                            p { "Welcome to AWG Messenger!" }
                        }
                    }
                    footer ."bg-gray-800 text-white p-4" {
                        p { "2023 AWG Messenger" }
                    }
                }
            },
        )
        .render()
        .into_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn should_render_tabs() {
        let expected = r##"<div class="flex flex-col w-full h-full" id="tabs" hx-get="/tabs/chats" hx-trigger="load" hx-target="#tabs" hx-swap="innerHTML" hx-ext="sse" sse-connect="/sse"></div>"##;

        let actual = Tabs.render().into_string();

        assert_eq!(actual, expected);
    }
}
