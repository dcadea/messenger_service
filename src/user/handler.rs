pub(super) mod api {
    use axum::{Extension, Form, extract::State};
    use maud::{Markup, Render, html};
    use serde::Deserialize;

    use crate::{
        auth, contact,
        user::{self, markup},
    };

    #[derive(Deserialize)]
    pub struct FindParams {
        nickname: String,
    }

    pub async fn search(
        auth_user: Extension<auth::User>,
        user_service: State<user::Service>,
        contact_service: State<contact::Service>,
        params: Form<FindParams>,
    ) -> crate::Result<Markup> {
        if params.nickname.is_empty() {
            return Ok(html! {(crate::markup::EMPTY)});
        }

        let users = user_service
            .search_user_info(&params.nickname, &auth_user)
            .await?;

        let contacts = contact_service
            .find_by_sub(auth_user.sub())
            .await
            .unwrap_or(Vec::with_capacity(0));

        Ok(markup::SearchResult::new(&contacts, &users).render())
    }
}
