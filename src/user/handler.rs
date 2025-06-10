use axum::http::StatusCode;

impl From<super::Error> for StatusCode {
    fn from(e: super::Error) -> Self {
        match e {
            super::Error::NotFound(_) => Self::NOT_FOUND,
            super::Error::MalformedPicture(_)
            | super::Error::MalformedEmail(_)
            | super::Error::_R2d2(_)
            | super::Error::_Diesel(_) => Self::INTERNAL_SERVER_ERROR,
        }
    }
}

pub(super) mod api {
    use axum::{Extension, Form, extract::State};
    use maud::{Markup, Render, html};
    use serde::Deserialize;

    use crate::{
        auth, contact,
        user::{self, Nickname, markup},
    };

    #[derive(Deserialize)]
    pub struct FindParams {
        nickname: Nickname,
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

        let users = user_service.search(&params.nickname, &auth_user)?;

        let contacts = contact_service
            .find_by_user_id(auth_user.id())
            .await
            .unwrap_or_else(|_| Vec::with_capacity(0));

        Ok(markup::SearchResult::new(&contacts, &users).render())
    }
}
