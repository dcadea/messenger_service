pub(super) mod pages {
    use axum::{
        Extension,
        extract::{Path, State},
    };
    use maud::{Markup, html};
    use messenger_service::markup::Wrappable;

    use crate::{
        talk::{self, markup, service::TalkService},
        user::model::UserInfo,
    };

    pub async fn home(
        user_info: Extension<UserInfo>,
        talk_service: State<TalkService>,
    ) -> crate::Result<Wrappable> {
        let talks = talk_service.find_all(&user_info).await?;
        Ok(Wrappable::new(markup::TalkWindow::new(&user_info, &talks)).with_sse())
    }

    pub async fn active_talk(
        id: Path<talk::Id>,
        logged_user: Extension<UserInfo>,
        talk_service: State<TalkService>,
    ) -> crate::Result<Markup> {
        let talk = &talk_service
            .find_by_id_and_sub(&id, &logged_user.sub)
            .await?;

        Ok(html! {(markup::ActiveTalk(&talk))})
    }
}

pub(super) mod api {
    use axum::{
        Extension,
        extract::{Path, State},
        response::IntoResponse,
    };
    use maud::{Markup, html};

    use crate::{
        talk::{self, service::TalkService},
        user::model::UserInfo,
    };

    pub async fn find_one(
        user_info: Extension<UserInfo>,
        talk_service: State<TalkService>,
        Path(id): Path<talk::Id>,
    ) -> crate::Result<Markup> {
        let talk = talk_service.find_by_id_and_sub(&id, &user_info.sub).await?;
        Ok(html! { (talk) })
    }

    pub async fn create() -> crate::Result<()> {
        todo!("implement create handler")
    }

    pub async fn delete(
        id: Path<talk::Id>,
        logged_user: Extension<UserInfo>,
        talk_service: State<TalkService>,
    ) -> crate::Result<impl IntoResponse> {
        talk_service.delete(&id, &logged_user).await?;

        Ok([("HX-Redirect", "/")])
    }
}
