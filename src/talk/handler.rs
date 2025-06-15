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
            | super::Error::UnsupportedKind(_)
            | super::Error::_User(_)
            | super::Error::_Integration(_)
            | super::Error::_R2d2(_)
            | super::Error::_Diesel(_) => Self::INTERNAL_SERVER_ERROR,
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
            .find_by_id_and_user_id(&id, auth_user.id())
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
        integration::storage::{self, Blob},
        talk::{self, markup},
        user,
    };

    pub async fn find_one(
        auth_user: Extension<auth::User>,
        talk_service: State<talk::Service>,
        Path(id): Path<talk::Id>,
    ) -> crate::Result<Markup> {
        let talk = talk_service
            .find_by_id_and_user_id(&id, auth_user.id())
            .await?;
        Ok(html! { (talk) })
    }

    pub async fn find_avatar(
        s3: State<storage::S3>,
        id: Path<talk::Id>,
    ) -> crate::Result<axum::body::Body> {
        // TODO: handle result
        let content = s3.find_one(Blob::Png(&id.0.to_string())).await.unwrap();
        let x = content.to_stream().await.unwrap().0;
        Ok(axum::body::Body::from_stream(x))
    }

    #[derive(Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    pub enum CreateParams {
        Chat { user_id: user::Id },
        Group(CreateGroupParams),
    }

    // This enum is needed to match the case when no users were selected
    // which results in a single value and deserialization into Vec is not possible
    #[derive(Deserialize)]
    #[serde(untagged)]
    pub enum CreateGroupParams {
        Valid {
            name: String,
            members: Vec<user::Id>,
        },
        Invalid {
            name: String,
            members: user::Id,
        },
    }

    #[axum::debug_handler]
    pub async fn create(
        Extension(auth_user): Extension<auth::User>,
        talk_service: State<talk::Service>,
        Json(params): Json<CreateParams>,
    ) -> crate::Result<Markup> {
        let auth_id = auth_user.id();
        let talk = match params {
            CreateParams::Chat { user_id } => talk_service.create_chat(auth_id, &user_id).await,
            CreateParams::Group(params) => {
                let (name, members) = match params {
                    CreateGroupParams::Valid { name, members } => (name, members),
                    CreateGroupParams::Invalid { name, members } => (name, vec![members]),
                };

                talk_service.create_group(auth_id, &name, &members).await
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

    pub async fn _get_avatar(_id: Path<talk::Id>, _s3: State<storage::S3>) {
        todo!()
    }
}

pub(super) mod templates {
    use axum::{Extension, extract::State};
    use maud::{Markup, Render};

    use crate::{auth, contact, talk, user};

    pub struct GroupMemberDto {
        user_id: user::Id,
        name: String,
        picture: String,
    }

    impl GroupMemberDto {
        pub fn new(user_id: user::Id, name: impl Into<String>, picture: impl Into<String>) -> Self {
            Self {
                user_id,
                name: name.into(),
                picture: picture.into(),
            }
        }

        pub const fn user_id(&self) -> &user::Id {
            &self.user_id
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
            .find_by_user_id_and_status(auth_user.id(), &contact::Status::Accepted)
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
