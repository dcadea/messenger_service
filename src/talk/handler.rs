use axum::http::StatusCode;

impl From<super::Error> for StatusCode {
    fn from(e: super::Error) -> Self {
        match e {
            super::Error::NotFound(_) => StatusCode::NOT_FOUND,
            super::Error::NotMember => StatusCode::FORBIDDEN,
            super::Error::AlreadyExists => StatusCode::CONFLICT,
            super::Error::NotEnoughMembers(_)
            | super::Error::MissingName
            | super::Error::NonExistingUser(_)
            | super::Error::UnsupportedStatus => StatusCode::BAD_REQUEST,
            super::Error::NotCreated
            | super::Error::NotDeleted
            | super::Error::_User(_)
            | super::Error::_MongoDB(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub(super) mod pages {
    use axum::{
        Extension,
        extract::{Path, State},
    };
    use maud::{Markup, html};

    use crate::{
        auth,
        talk::{self, markup},
    };

    pub async fn active_talk(
        id: Path<talk::Id>,
        auth_user: Extension<auth::User>,
        talk_service: State<talk::Service>,
    ) -> crate::Result<Markup> {
        let talk = &talk_service
            .find_by_id_and_sub(&id, auth_user.sub())
            .await?;

        Ok(html! {(markup::ActiveTalk(&talk))})
    }
}

pub(super) mod api {
    use axum::{
        Extension, Json,
        extract::{Path, State},
        response::IntoResponse,
    };
    use maud::{Markup, html};
    use serde::Deserialize;

    use crate::{
        auth,
        talk::{self, markup},
        user,
    };

    pub async fn find_one(
        auth_user: Extension<auth::User>,
        talk_service: State<talk::Service>,
        Path(id): Path<talk::Id>,
    ) -> crate::Result<Markup> {
        let talk = talk_service
            .find_by_id_and_sub(&id, auth_user.sub())
            .await?;
        Ok(html! { (talk) })
    }

    #[derive(Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    pub enum CreateParams {
        Chat { sub: user::Sub },
        Group(CreateGroupParams),
    }

    // This enum is needed to match the case when no users were selected
    // which results in a single value and deserialization into Vec is not possible
    #[derive(Deserialize)]
    #[serde(untagged)]
    pub enum CreateGroupParams {
        Valid {
            name: String,
            members: Vec<user::Sub>,
        },
        Invalid {
            name: String,
            members: user::Sub,
        },
    }

    #[axum::debug_handler]
    pub async fn create(
        Extension(auth_user): Extension<auth::User>,
        talk_service: State<talk::Service>,
        Json(params): Json<CreateParams>,
    ) -> crate::Result<Markup> {
        let auth_sub = auth_user.sub();
        let talk = match params {
            CreateParams::Chat { sub } => talk_service.create_chat(auth_sub, &sub).await,
            CreateParams::Group(params) => {
                let (name, members) = match params {
                    CreateGroupParams::Valid { name, members } => (name, members),
                    CreateGroupParams::Invalid { name, members } => (name, vec![members]),
                };

                talk_service.create_group(auth_sub, &name, &members).await
            }
        }?;

        Ok(html! {(markup::ActiveTalk(&talk))})
    }

    pub async fn delete(
        id: Path<talk::Id>,
        auth_user: Extension<auth::User>,
        talk_service: State<talk::Service>,
    ) -> crate::Result<impl IntoResponse> {
        talk_service.delete(&id, &auth_user).await?;

        Ok([("HX-Redirect", "/")])
    }
}

pub(super) mod templates {
    use axum::Extension;
    use maud::{Markup, Render};

    use crate::{auth, talk};

    pub async fn create_group(auth_user: Extension<auth::User>) -> Markup {
        talk::markup::CreateGroupForm::new(&auth_user).render()
    }
}
