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

struct MainContent<'a>(&'a Markup);

impl Render for MainContent<'_> {
    fn render(&self) -> Markup {
        html! {
            div ."main-content"
                ."max-w-lg h-3/5 md:h-4/5 md:w-4/5 w-full h-full"
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

            body ."h-screen bg-black flex items-center justify-center"
                hx-ext=[w.hx_ext_sse()]
                sse-connect=[w.sse_connect()]
            {
                (MainContent(&w.content))
            }
        }
    }
}

#[derive(Clone)]
pub struct Wrappable {
    content: Markup,
    sse: bool,
}

impl Wrappable {
    pub fn new(content: impl Render) -> Self {
        Self {
            content: content.render(),
            sse: false,
        }
    }

    #[must_use]
    pub fn with_sse(mut self) -> Self {
        self.sse = true;
        self
    }

    fn hx_ext_sse(&self) -> Option<&str> {
        if self.sse { Some("sse") } else { None }
    }

    fn sse_connect(&self) -> Option<&str> {
        if self.sse { Some("/sse") } else { None }
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

enum TabControl<'a> {
    Chats(&'a SelectedTab),
    Groups(&'a SelectedTab),
    Contacts(&'a SelectedTab),
    Settings(&'a SelectedTab),
}

impl Render for TabControl<'_> {
    fn render(&self) -> Markup {
        let (path, selected, i_class) = match self {
            TabControl::Chats(st) => (
                "/tabs/chats",
                SelectedTab::Chats.eq(st),
                "fa-regular fa-message",
            ),
            TabControl::Groups(st) => (
                "/tabs/groups",
                SelectedTab::Groups.eq(st),
                "fa-solid fa-people-group",
            ),
            TabControl::Contacts(st) => (
                "/tabs/contacts",
                SelectedTab::Contacts.eq(st),
                "fa-regular fa-address-book",
            ),
            TabControl::Settings(st) => (
                "/tabs/settings",
                SelectedTab::Settings.eq(st),
                "fa-solid fa-gears",
            ),
        };

        html! {
            button ."basis-64 py-4 hover:bg-gray-300 cursor-pointer"
                hx-get=(path)
                role="tab"
                .bg-gray-100[selected]
                aria-selected=(selected)
                aria-controls="tab-content"
            {
                i .(i_class) {}
            }
        }
    }
}

#[derive(PartialEq)]
pub enum SelectedTab {
    Chats,
    Groups,
    Contacts,
    Settings,
}

impl Render for SelectedTab {
    fn render(&self) -> Markup {
        html! {
            div ."flex flex-row text-2xl" role="tablist" {
                (TabControl::Chats(self))
                (TabControl::Groups(self))
                (TabControl::Contacts(self))
                (TabControl::Settings(self))
            }
        }
    }
}

pub struct Tab {
    selected: SelectedTab,
    content: Markup,
}

impl Tab {
    pub fn new(selected: SelectedTab, content: impl Render) -> Self {
        Self {
            selected,
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

            (self.selected)
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
                hx-swap="innerHTML" {}
        }
    }
}
