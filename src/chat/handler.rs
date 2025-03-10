pub(super) mod pages {
    use axum::{
        Extension,
        extract::{Path, State},
    };
    use maud::{Markup, Render};
    use messenger_service::markup::Wrappable;

    use crate::{
        chat,
        user::{model::UserInfo, service::UserService},
    };

    use crate::chat::{markup, service::ChatService};

    pub async fn home(
        user_info: Extension<UserInfo>,
        chat_service: State<ChatService>,
    ) -> crate::Result<Wrappable> {
        let chats = chat_service.find_all(&user_info).await?;
        Ok(Wrappable::new(markup::ChatWindow::new(&user_info, &chats)).with_sse())
    }

    pub async fn active_chat(
        id: Path<chat::Id>,
        logged_user: Extension<UserInfo>,
        chat_service: State<ChatService>,
        user_service: State<UserService>,
    ) -> crate::Result<Markup> {
        let chat = &chat_service.find_by_id(&id, &logged_user).await?;

        let id = &chat.id;
        let recipient = &user_service.find_user_info(&chat.recipient).await?;

        Ok(markup::ActiveChat { id, recipient }.render())
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
        chat::{self, Id, markup, service::ChatService},
        user::{self, model::UserInfo, service::UserService},
    };

    pub async fn find_one(
        user_info: Extension<UserInfo>,
        chat_service: State<ChatService>,
        Path(id): Path<Id>,
    ) -> crate::Result<Markup> {
        let chat = chat_service.find_by_id(&id, &user_info).await?;
        Ok(html! { (chat) })
    }

    #[derive(Deserialize)]
    pub struct CreateParams {
        kind: chat::Kind,
        sub: user::Sub,
    }

    pub async fn create(
        Extension(logged_user): Extension<UserInfo>,
        chat_service: State<ChatService>,
        user_service: State<UserService>,
        Form(params): Form<CreateParams>,
    ) -> crate::Result<Markup> {
        let recipient = &params.sub;
        let chat = chat_service
            .create(&logged_user, &params.kind, recipient)
            .await?;

        let id = &chat.id;
        let recipient = &user_service.find_user_info(recipient).await?;

        Ok(html! {(markup::ActiveChat { id, recipient })})
    }

    pub async fn delete(
        chat_id: Path<Id>,
        logged_user: Extension<UserInfo>,
        chat_service: State<ChatService>,
    ) -> crate::Result<impl IntoResponse> {
        chat_service.delete(&chat_id, &logged_user).await?;

        Ok([("HX-Redirect", "/")])
    }
}
