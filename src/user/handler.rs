pub(super) mod api {
    use axum::{Extension, Form, extract::State};
    use maud::{Markup, Render, html};
    use serde::Deserialize;

    use crate::user::{markup, model::UserInfo, service::UserService};

    #[derive(Deserialize)]
    pub struct FindParams {
        nickname: String,
    }

    pub async fn search(
        user_info: Extension<UserInfo>,
        user_service: State<UserService>,
        params: Form<FindParams>,
    ) -> crate::Result<Markup> {
        if params.nickname.is_empty() {
            return Ok(html! {(messenger_service::markup::EMPTY)});
        }

        let users = user_service
            .search_user_info(&params.nickname, &user_info.nickname)
            .await?;

        let friends = user_service
            .find_friends(&user_info.sub)
            .await
            .unwrap_or(user_info.friends.clone());

        Ok(markup::SearchResult::new(&friends, &users).render())
    }
}
