use crate::markup::{SelectedTab, Tab, Wrappable};
use axum::{Extension, extract::State, response::IntoResponse};
use maud::{Render, html};

use crate::{talk, user::model::UserInfo};

pub async fn home(
    _user_info: Extension<UserInfo>,
    _talk_service: State<talk::Service>,
) -> crate::Result<Wrappable> {
    // first shown component is chats page
    Ok(Wrappable::new(html! {
        #tabs hx-get="/tabs/chats" hx-trigger="load" hx-target="#tabs" hx-swap="innerHTML" {}
    })
    .with_sse())
}

// GET /tabs/chats
pub async fn chats_tab() -> impl IntoResponse {
    // TODO: filter by details == chat
    // let talks = talk_service.find_all(&user_info).await?;
    Tab::new(SelectedTab::Chats, html! {"chats"})
        .render()
        .into_response()
}

// GET /tabs/groups
pub async fn groups_tab() -> impl IntoResponse {
    // TODO: filter by details == group
    // let talks = talk_service.find_all(&user_info).await?;
    Tab::new(SelectedTab::Groups, html! {"groups"})
        .render()
        .into_response()
}

// GET /tabs/contacts
pub async fn contacts_tab() -> impl IntoResponse {
    Tab::new(SelectedTab::Contacts, html! {"contacts"})
        .render()
        .into_response()
}

// GET /tabs/settings
pub async fn settings_tab() -> impl IntoResponse {
    Tab::new(SelectedTab::Settings, html! {"settings"})
        .render()
        .into_response()
}
