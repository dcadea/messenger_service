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
        let talk = &talk_service.find_by_id_and_sub(&id, &auth_user.sub).await?;

        Ok(html! {(markup::ActiveTalk(&talk))})
    }
}

pub(super) mod api {
    use axum::{
        Extension, Form,
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
        let talk = talk_service.find_by_id_and_sub(&id, &auth_user.sub).await?;
        Ok(html! { (talk) })
    }

    #[derive(Deserialize)]
    #[serde(untagged, rename_all = "snake_case")]
    pub enum CreateParams {
        Chat {
            sub: user::Sub,
        },
        Group {
            name: String,
            members: Vec<user::Sub>,
        },
    }

    pub async fn create(
        Extension(auth_user): Extension<auth::User>,
        talk_service: State<talk::Service>,
        Form(params): Form<CreateParams>,
    ) -> crate::Result<Markup> {
        let auth_sub = &auth_user.sub;
        let talk = match params {
            CreateParams::Chat { sub } => talk_service.create_chat(auth_sub, &sub).await,
            CreateParams::Group { name, members } => {
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
