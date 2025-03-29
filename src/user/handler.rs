pub(super) mod api {
    use std::collections::HashSet;

    use axum::{Extension, Form, extract::State};
    use maud::{Markup, Render, html};
    use serde::Deserialize;

    use crate::{
        contact,
        user::{self, markup, model::UserInfo},
    };

    #[derive(Deserialize)]
    pub struct FindParams {
        nickname: String,
    }

    pub async fn search(
        user_info: Extension<UserInfo>,
        user_service: State<user::Service>,
        contact_service: State<contact::Service>,
        params: Form<FindParams>,
    ) -> crate::Result<Markup> {
        if params.nickname.is_empty() {
            return Ok(html! {(messenger_service::markup::EMPTY)});
        }

        let users = user_service
            .search_user_info(&params.nickname, &user_info)
            .await?;

        let contacts = contact_service
            .find_contact_subs(&user_info.sub)
            .await
            .unwrap_or(HashSet::with_capacity(0));

        Ok(markup::SearchResult::new(&contacts, &users).render())
    }
}
