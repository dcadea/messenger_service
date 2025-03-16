pub(super) mod api {
    use axum::{
        Extension, Form,
        extract::{Path, State},
        response::IntoResponse,
    };
    use maud::Markup;
    use serde::Deserialize;

    use crate::{
        chat::{self, Id, service::ChatService},
        user::{self, model::UserInfo, service::UserService},
    };

    #[derive(Deserialize)]
    pub struct CreateParams {
        kind: chat::Kind,
        sub: user::Sub,
    }

    pub async fn create(
        Extension(_logged_user): Extension<UserInfo>,
        _chat_service: State<ChatService>,
        _user_service: State<UserService>,
        Form(_params): Form<CreateParams>,
    ) -> crate::Result<Markup> {
        // let recipient = &params.sub;
        // let chat = chat_service
        //     .create(&logged_user, params.kind, recipient)
        //     .await?;

        // let id = &chat.id;
        // let recipient = &user_service.find_user_info(recipient).await?;

        // Ok(html! {(markup::ActiveChat { id, recipient })})
        panic!("deprecated")
    }
}
