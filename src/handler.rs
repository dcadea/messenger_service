use crate::{
    markup::{SelectedTab, Tab, Tabs, Wrappable},
    talk::markup::TalkWindow,
};
use axum::{Extension, extract::State, response::IntoResponse};
use maud::{Markup, Render, html};

use crate::{talk, user::model::UserInfo};

pub async fn home(
    _user_info: Extension<UserInfo>,
    _talk_service: State<talk::Service>,
) -> crate::Result<Wrappable> {
    // first shown component is chats page
    Ok(Wrappable::new(Tabs {}).with_sse())
}

// GET /tabs/chats
pub async fn chats_tab(
    logged_sub: Extension<UserInfo>,
    talk_service: State<talk::Service>,
) -> crate::Result<Markup> {
    let chats = talk_service
        .find_all_by_kind(&logged_sub, &talk::Kind::Chat)
        .await?;

    let tab_content = TalkWindow::chat(&logged_sub, &chats);
    Ok(Tab::new(SelectedTab::Chats, tab_content).render())
}

// GET /tabs/groups
pub async fn groups_tab(
    logged_sub: Extension<UserInfo>,
    talk_service: State<talk::Service>,
) -> crate::Result<Markup> {
    let groups = talk_service
        .find_all_by_kind(&logged_sub, &talk::Kind::Group)
        .await?;

    let tab_content = TalkWindow::group(&logged_sub, &groups);
    Ok(Tab::new(SelectedTab::Groups, tab_content).render())
}

// GET /tabs/contacts
pub async fn contacts_tab() -> impl IntoResponse {
    // TODO
    Tab::new(SelectedTab::Contacts, html! {"contacts"})
        .render()
        .into_response()
}

// GET /tabs/settings
pub async fn settings_tab() -> impl IntoResponse {
    // TODO
    Tab::new(SelectedTab::Settings, html! {"settings"})
        .render()
        .into_response()
}
