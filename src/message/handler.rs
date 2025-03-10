pub(super) mod api {
    use axum::extract::{Path, State};
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::{Extension, Form};
    use axum_extra::extract::Query;
    use maud::{Markup, Render};
    use serde::Deserialize;

    use crate::chat::service::{ChatService, ChatValidator};
    use crate::error::Error;
    use crate::user::model::UserInfo;
    use crate::{chat, message, user};

    use crate::message::markup;
    use crate::message::model::{LastMessage, Message};
    use crate::message::service::MessageService;

    #[derive(Deserialize)]
    pub struct CreateParams {
        chat_id: chat::Id,
        recipient: user::Sub,
        text: String,
    }

    pub async fn create(
        user_info: Extension<UserInfo>,
        message_service: State<MessageService>,
        chat_service: State<ChatService>,
        Form(params): Form<CreateParams>,
    ) -> crate::Result<Markup> {
        let msg = Message::new(
            params.chat_id,
            user_info.sub.clone(),
            params.recipient,
            params.text.trim(),
        );

        let msgs = message_service.create(&msg).await?;

        if let Some(last) = msgs.last() {
            let last_msg = LastMessage::from(last);
            chat_service
                .update_last_message(&last.chat_id, Some(&last_msg))
                .await?;
        }

        Ok(markup::MessageList::prepend(&msgs, &user_info.sub).render())
    }

    #[derive(Deserialize)]
    pub struct FindAllParams {
        chat_id: Option<chat::Id>,
        end_time: Option<i64>,
        limit: Option<usize>,
    }

    pub async fn find_all(
        user_info: Extension<UserInfo>,
        Query(params): Query<FindAllParams>,
        chat_validator: State<ChatValidator>,
        chat_service: State<ChatService>,
        message_service: State<MessageService>,
    ) -> crate::Result<impl IntoResponse> {
        let chat_id = params
            .chat_id
            .ok_or(Error::QueryParamRequired("chat_id".to_owned()))?;

        let logged_sub = &user_info.sub;

        chat_validator.check_member(&chat_id, logged_sub).await?;

        let (msgs, seen_qty) = message_service
            .find_by_chat_id_and_params(logged_sub, &chat_id, params.limit, params.end_time)
            .await?;

        if seen_qty > 0 {
            chat_service.mark_as_seen(&chat_id).await?;
        }

        Ok(markup::MessageList::append(&msgs, logged_sub).render())
    }

    #[derive(Deserialize)]
    pub struct UpdateParams {
        message_id: message::Id,
        text: String,
    }

    pub async fn update(
        user_info: Extension<UserInfo>,
        message_service: State<MessageService>,
        Form(params): Form<UpdateParams>,
    ) -> crate::Result<impl IntoResponse> {
        let msg = message_service
            .update(&user_info.sub, &params.message_id, &params.text)
            .await?;

        Ok((
            StatusCode::OK,
            [("HX-Trigger", "msg:afterUpdate")],
            markup::MessageItem::new(&msg, Some(&user_info.sub)).render(),
        ))
    }

    pub async fn delete(
        user_info: Extension<UserInfo>,
        Path(id): Path<message::Id>,
        message_service: State<MessageService>,
        chat_service: State<ChatService>,
    ) -> crate::Result<()> {
        if let Some(deleted_msg) = message_service.delete(&user_info.sub, &id).await? {
            let is_last = chat_service.is_last_message(&deleted_msg).await?;
            if is_last {
                let chat_id = &deleted_msg.chat_id;
                let last_msg = message_service
                    .find_most_recent(chat_id)
                    .await?
                    .map(|msg| LastMessage::from(&msg));

                chat_service
                    .update_last_message(chat_id, last_msg.as_ref())
                    .await?;
            }

            return Ok(());
        }

        Err(message::Error::NotFound(Some(id)))?
    }
}

pub(super) mod templates {
    use axum::{
        Extension,
        extract::{Query, State},
    };
    use maud::{Markup, Render};
    use serde::Deserialize;

    use crate::{
        chat,
        message::{self, markup, service::MessageService},
        user::{self, model::UserInfo},
    };

    #[derive(Deserialize)]
    pub struct BlankParams {
        chat_id: chat::Id,
        recipient: user::Sub,
    }

    pub async fn message_input_blank(params: Query<BlankParams>) -> Markup {
        markup::InputBlank::new(&params.chat_id, &params.recipient).render()
    }

    #[derive(Deserialize)]
    pub struct EditParams {
        message_id: message::Id,
    }

    pub async fn message_input_edit(
        user_info: Extension<UserInfo>,
        params: Query<EditParams>,
        message_service: State<MessageService>,
    ) -> crate::Result<Markup> {
        let msg = message_service.find_by_id(&params.message_id).await?;

        if msg.owner != user_info.sub {
            return Err(crate::error::Error::from(message::Error::NotOwner));
        }

        Ok(markup::InputEdit::new(&msg._id, &msg.text).render())
    }
}
