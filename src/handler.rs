use crate::{
    auth,
    contact::{self, markup::ContactInfos, model::ContactDto},
    markup::{SelectedTab, Tab, Tabs, Wrappable},
    settings,
    talk::markup::TalkWindow,
    user,
};
use axum::{Extension, extract::State};
use maud::{Markup, Render};

use crate::{talk, user::model::UserInfo};

// first shown component is chats page
pub async fn home() -> crate::Result<Wrappable> {
    Ok(Wrappable::new(Tabs {}).with_sse())
}

// GET /tabs/chats
pub async fn chats_tab(
    auth_user: Extension<auth::User>,
    talk_service: State<talk::Service>,
) -> crate::Result<Markup> {
    let chats = talk_service
        .find_all_by_kind(&auth_user, &talk::Kind::Chat)
        .await?;

    let tab_content = TalkWindow::chats(&auth_user, &chats);
    Ok(Tab::new(SelectedTab::Chats, tab_content).render())
}

// GET /tabs/groups
pub async fn groups_tab(
    auth_user: Extension<auth::User>,
    talk_service: State<talk::Service>,
) -> crate::Result<Markup> {
    let groups = talk_service
        .find_all_by_kind(&auth_user, &talk::Kind::Group)
        .await?;

    let tab_content = TalkWindow::groups(&auth_user, &groups);
    Ok(Tab::new(SelectedTab::Groups, tab_content).render())
}

// GET /tabs/contacts
pub async fn contacts_tab(
    auth_user: Extension<auth::User>,
    contact_service: State<contact::Service>,
    user_service: State<user::Service>,
) -> crate::Result<Markup> {
    let contacts = contact_service.find_by_sub(auth_user.sub()).await?;

    let mut contact_infos: Vec<(ContactDto, UserInfo)> = Vec::with_capacity(contacts.len());
    for c in contacts {
        let ui = user_service.find_user_info(&c.recipient).await?;
        contact_infos.push((c, ui));
    }

    Ok(Tab::new(
        SelectedTab::Contacts,
        ContactInfos::new(&auth_user, &contact_infos),
    )
    .render())
}

// GET /tabs/settings
pub async fn settings_tab() -> Markup {
    Tab::new(SelectedTab::Settings, settings::List).render()
}
