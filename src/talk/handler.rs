pub(super) mod pages {
    use axum::{
        Extension,
        extract::{Path, State},
    };
    use maud::{Markup, html};

    use crate::{
        talk::{self, markup},
        user::model::UserInfo,
    };

    pub async fn active_talk(
        id: Path<talk::Id>,
        logged_user: Extension<UserInfo>,
        talk_service: State<talk::Service>,
    ) -> crate::Result<Markup> {
        let talk = &talk_service
            .find_by_id_and_sub(&id, &logged_user.sub)
            .await?;

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
        talk::{self, markup},
        user::{self, model::UserInfo},
    };

    pub async fn find_one(
        user_info: Extension<UserInfo>,
        talk_service: State<talk::Service>,
        Path(id): Path<talk::Id>,
    ) -> crate::Result<Markup> {
        let talk = talk_service.find_by_id_and_sub(&id, &user_info.sub).await?;
        Ok(html! { (talk) })
    }

    #[derive(Deserialize)]
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
        Extension(logged_user): Extension<UserInfo>,
        talk_service: State<talk::Service>,
        Form(params): Form<CreateParams>,
    ) -> crate::Result<Markup> {
        let logged_sub = &logged_user.sub;
        let talk = match params {
            CreateParams::Chat { sub } => talk_service.create_chat(logged_sub, &sub).await,
            CreateParams::Group { name, members } => {
                talk_service.create_group(logged_sub, &name, &members).await
            }
        }?;

        Ok(html! {(markup::ActiveTalk(&talk))})
    }

    pub async fn delete(
        id: Path<talk::Id>,
        logged_user: Extension<UserInfo>,
        talk_service: State<talk::Service>,
    ) -> crate::Result<impl IntoResponse> {
        talk_service.delete(&id, &logged_user).await?;

        Ok([("HX-Redirect", "/")])
    }
}
