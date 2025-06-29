use axum::http::StatusCode;

impl From<super::Error> for StatusCode {
    fn from(e: super::Error) -> Self {
        match e {
            super::Error::NotFound(_) => Self::NOT_FOUND,
            super::Error::EmptyContent => Self::BAD_REQUEST,
            super::Error::IdNotPresent
            | super::Error::Unexpected(_)
            | super::Error::_User(_)
            | super::Error::_R2d2(_)
            | super::Error::_Diesel(_) => Self::INTERNAL_SERVER_ERROR,
        }
    }
}

pub(super) mod api {
    use axum::extract::{Path, State};
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::{Extension, Form};
    use axum_extra::extract::Query;
    use maud::{Markup, Render};
    use serde::Deserialize;

    use crate::error::Error;
    use crate::{auth, message, talk, user};

    use crate::message::markup;

    #[derive(Deserialize)]
    pub struct CreateParams {
        talk_id: talk::Id,
        text: String,
    }

    pub async fn create(
        auth_user: Extension<auth::User>,
        message_service: State<message::Service>,
        Form(params): Form<CreateParams>,
    ) -> crate::Result<Markup> {
        let msgs = message_service
            .create(&params.talk_id, &auth_user, params.text.trim())
            .await?;

        Ok(markup::MessageList::prepend(&msgs, auth_user.id()).render())
    }

    #[derive(Deserialize)]
    pub struct FindAllParams {
        talk_id: Option<talk::Id>,
        _end_time: Option<i64>,
        limit: Option<i64>,
    }

    pub async fn find_all(
        auth_user: Extension<auth::User>,
        Query(params): Query<FindAllParams>,
        user_service: State<user::Service>,
        message_service: State<message::Service>,
    ) -> crate::Result<impl IntoResponse> {
        let talk_id = params
            .talk_id
            .ok_or(Error::QueryParamRequired("talk_id".to_owned()))?;

        user_service.check_member(&talk_id, &auth_user).await?;

        let msgs = message_service
            .find_by_talk_id_and_params(
                &auth_user,
                &talk_id,
                params.limit,
                None, /* FIXME: params.end_time*/
            )
            .await?;

        Ok(markup::MessageList::append(&msgs, auth_user.id()).render())
    }

    #[derive(Deserialize)]
    pub struct UpdateParams {
        message_id: message::Id,
        text: String,
    }

    pub async fn update(
        auth_user: Extension<auth::User>,
        message_service: State<message::Service>,
        Form(params): Form<UpdateParams>,
    ) -> crate::Result<impl IntoResponse> {
        let msg = message_service
            .update(&auth_user, &params.message_id, &params.text)
            .await?;

        Ok((
            StatusCode::OK,
            [("HX-Trigger", "msg:afterUpdate")],
            markup::MessageItem::new(&msg, Some(auth_user.id())).render(),
        ))
    }

    pub async fn delete(
        auth_user: Extension<auth::User>,
        Path(id): Path<message::Id>,
        message_service: State<message::Service>,
    ) -> crate::Result<()> {
        // FIXME: "update or delete on table \"messages\" violates foreign key constraint \"fk_last_message\" on table \"talks\""
        // happens when message that is deleted is "last"
        if let Some(_) = message_service.delete(&auth_user, &id).await? {
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
        auth,
        message::{self, markup},
        talk,
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
        auth_user: Extension<auth::User>,
        params: Query<EditParams>,
        message_service: State<message::Service>,
    ) -> crate::Result<Markup> {
        let msg = message_service.find_by_id(&auth_user, &params.message_id)?;

        Ok(markup::InputEdit::new(msg.id(), msg.text()).render())
    }
}
