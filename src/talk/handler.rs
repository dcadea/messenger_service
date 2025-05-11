use axum::http::StatusCode;

impl From<super::Error> for StatusCode {
    fn from(e: super::Error) -> Self {
        match e {
            super::Error::NotFound(_) => Self::NOT_FOUND,
            super::Error::NotMember => Self::FORBIDDEN,
            super::Error::AlreadyExists => Self::CONFLICT,
            super::Error::NotEnoughMembers(_)
            | super::Error::MissingName
            | super::Error::NonExistingUser(_)
            | super::Error::UnsupportedStatus => Self::BAD_REQUEST,
            super::Error::NotCreated
            | super::Error::NotDeleted
            | super::Error::_User(_)
            | super::Error::_MongoDB(_) => Self::INTERNAL_SERVER_ERROR,
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

        Ok(html! {(markup::ActiveTalk(&auth_user, &talk))})
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
        user::Sub,
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
        Chat { sub: Sub },
        Group(CreateGroupParams),
    }

    // This enum is needed to match the case when no users were selected
    // which results in a single value and deserialization into Vec is not possible
    #[derive(Deserialize)]
    #[serde(untagged)]
    pub enum CreateGroupParams {
        Valid { name: String, members: Vec<Sub> },
        Invalid { name: String, members: Sub },
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

        Ok(html! {(markup::ActiveTalk(&auth_user, &talk))})
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
    use axum::{Extension, extract::State};
    use maud::{Markup, Render};

    use crate::{
        auth, contact, talk,
        user::{self, Sub},
    };

    pub struct GroupMemberDto {
        sub: Sub,
        name: String,
        picture: String,
    }

    impl GroupMemberDto {
        pub fn new(sub: Sub, name: impl Into<String>, picture: impl Into<String>) -> Self {
            Self {
                sub,
                name: name.into(),
                picture: picture.into(),
            }
        }

        pub const fn sub(&self) -> &Sub {
            &self.sub
        }

        pub fn name(&self) -> &str {
            &self.name
        }

        pub fn picture(&self) -> &str {
            &self.picture
        }
    }

    pub async fn create_group(
        auth_user: Extension<auth::User>,
        contact_service: State<contact::Service>,
        user_service: State<user::Service>,
    ) -> crate::Result<Markup> {
        let contacts = contact_service
            .find_by_sub_and_status(auth_user.sub(), &contact::Status::Accepted)
            .await?;

        let mut members: Vec<GroupMemberDto> = Vec::with_capacity(contacts.len());
        for c in contacts {
            let name = user_service.find_name(c.recipient()).await?;
            let picture = user_service.find_picture(c.recipient()).await?;
            members.push(GroupMemberDto::new(
                c.recipient().clone(),
                name,
                picture.to_string(),
            ));
        }

        Ok(talk::markup::CreateGroupForm::new(&auth_user, &members).render())
    }
}
