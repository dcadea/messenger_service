use crate::{
    contact::{self, markup::ContactInfos},
    markup::{SelectedTab, Tab, Tabs, Wrappable},
    talk::markup::TalkWindow,
    user,
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
    logged_user: Extension<UserInfo>,
    talk_service: State<talk::Service>,
) -> crate::Result<Markup> {
    let chats = talk_service
        .find_all_by_kind(&logged_user, &talk::Kind::Chat)
        .await?;

    let tab_content = TalkWindow::chat(&logged_user, &chats);
    Ok(Tab::new(SelectedTab::Chats, tab_content).render())
}

// GET /tabs/groups
pub async fn groups_tab(
    logged_user: Extension<UserInfo>,
    talk_service: State<talk::Service>,
) -> crate::Result<Markup> {
    let groups = talk_service
        .find_all_by_kind(&logged_user, &talk::Kind::Group)
        .await?;

    let tab_content = TalkWindow::group(&logged_user, &groups);
    Ok(Tab::new(SelectedTab::Groups, tab_content).render())
}

// GET /tabs/contacts
pub async fn contacts_tab(
    logged_user: Extension<UserInfo>,
    contact_service: State<contact::Service>,
    user_service: State<user::Service>,
) -> crate::Result<Markup> {
    let contacts = contact_service.find_contact_subs(&logged_user.sub).await?;

    let mut contact_infos: Vec<UserInfo> = Vec::with_capacity(contacts.len());
    for c in contacts {
        let ui = user_service.find_user_info(&c).await?;
        contact_infos.push(ui);
    }

    Ok(Tab::new(SelectedTab::Contacts, ContactInfos(&contact_infos)).render())
}

// GET /tabs/settings
pub async fn settings_tab() -> impl IntoResponse {
    // TODO
    Tab::new(SelectedTab::Settings, html! {"settings"})
        .render()
        .into_response()
}
