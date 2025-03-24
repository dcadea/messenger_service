pub(super) mod api {
    use axum::extract::{Path, State};
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::{Extension, Form};
    use axum_extra::extract::Query;
    use maud::{Markup, Render};
    use serde::Deserialize;

    use crate::error::Error;
    use crate::talk::service::TalkValidator;
    use crate::user::model::UserInfo;
    use crate::{message, talk};

    use crate::message::markup;
    use crate::message::model::{LastMessage, Message};

    #[derive(Deserialize)]
    pub struct CreateParams {
        talk_id: talk::Id,
        text: String,
    }

    pub async fn create(
        user_info: Extension<UserInfo>,
        message_service: State<message::Service>,
        talk_service: State<talk::Service>,
        Form(params): Form<CreateParams>,
    ) -> crate::Result<Markup> {
        let msg = Message::new(params.talk_id, user_info.sub.clone(), params.text.trim());

        let msgs = message_service.create(&msg).await?;

        if let Some(last) = msgs.last() {
            let last_msg = LastMessage::from(last);
            talk_service
                .update_last_message(&last.talk_id, Some(&last_msg))
                .await?;
        }

        Ok(markup::MessageList::prepend(&msgs, &user_info.sub).render())
    }

    #[derive(Deserialize)]
    pub struct FindAllParams {
        talk_id: Option<talk::Id>,
        end_time: Option<i64>,
        limit: Option<usize>,
    }

    pub async fn find_all(
        user_info: Extension<UserInfo>,
        Query(params): Query<FindAllParams>,
        talk_validator: State<TalkValidator>,
        talk_service: State<talk::Service>,
        message_service: State<message::Service>,
    ) -> crate::Result<impl IntoResponse> {
        let talk_id = params
            .talk_id
            .ok_or(Error::QueryParamRequired("talk_id".to_owned()))?;

        let logged_sub = &user_info.sub;

        talk_validator.check_member(&talk_id, logged_sub).await?;

        let (msgs, seen_qty) = message_service
            .find_by_talk_id_and_params(logged_sub, &talk_id, params.limit, params.end_time)
            .await?;

        if seen_qty > 0 {
            talk_service.mark_as_seen(&talk_id).await?;
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
        message_service: State<message::Service>,
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
        message_service: State<message::Service>,
        talk_service: State<talk::Service>,
    ) -> crate::Result<()> {
        if let Some(deleted_msg) = message_service.delete(&user_info.sub, &id).await? {
            let is_last = message_service.is_last_message(&deleted_msg).await?;
            if is_last {
                let talk_id = &deleted_msg.talk_id;
                let last_msg = message_service
                    .find_most_recent(talk_id)
                    .await?
                    .map(|msg| LastMessage::from(&msg));

                talk_service
                    .update_last_message(talk_id, last_msg.as_ref())
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
        message::{self, markup},
        talk,
        user::model::UserInfo,
    };

    #[derive(Deserialize)]
    pub struct BlankParams {
        talk_id: talk::Id,
    }

    pub async fn message_input_blank(params: Query<BlankParams>) -> Markup {
        markup::InputBlank(&params.talk_id).render()
    }

    #[derive(Deserialize)]
    pub struct EditParams {
        message_id: message::Id,
    }

    pub async fn message_input_edit(
        user_info: Extension<UserInfo>,
        params: Query<EditParams>,
        message_service: State<message::Service>,
    ) -> crate::Result<Markup> {
        let msg = message_service.find_by_id(&params.message_id).await?;

        if msg.owner != user_info.sub {
            return Err(crate::error::Error::from(message::Error::NotOwner));
        }

        Ok(markup::InputEdit::new(&msg._id, &msg.text).render())
    }
}
