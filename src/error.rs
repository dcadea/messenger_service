use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use log::error;
use maud::{html, Markup, Render};
use serde::Serialize;

use crate::{auth, chat, event, message, user};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Query parameter '{0}' is required")]
    QueryParamRequired(String),

    #[error(transparent)]
    _Auth(#[from] auth::Error),
    #[error(transparent)]
    _Chat(#[from] chat::Error),
    #[error(transparent)]
    _Event(#[from] event::Error),
    #[error(transparent)]
    _Message(#[from] message::Error),
    #[error(transparent)]
    _User(#[from] user::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let error_message = self.to_string();

        let (status, message) = match self {
            Self::_Auth(auth) => return auth.into_response(),
            Self::_Chat(chat) => return chat.into_response(),

            Self::_Event(event::Error::NotOwner) => (StatusCode::FORBIDDEN, error_message),
            Self::_Event(event::Error::NotRecipient) => (StatusCode::FORBIDDEN, error_message),

            Self::_Message(message::Error::NotFound(_)) => (StatusCode::NOT_FOUND, error_message),
            Self::_Message(message::Error::NotOwner) => (StatusCode::BAD_REQUEST, error_message),
            Self::_Message(message::Error::EmptyText) => (StatusCode::BAD_REQUEST, error_message),

            Self::_User(user::Error::NotFound(_)) => (StatusCode::NOT_FOUND, error_message),

            Self::QueryParamRequired(_) => (StatusCode::BAD_REQUEST, error_message),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_owned(),
            ),
        };

        error!("{self}");

        let mut header_map = HeaderMap::new();
        header_map.insert("HX-Retarget", HeaderValue::from_static("#errors"));
        (status, header_map, (ErrorResponse { message }).render()).into_response()
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
}

impl Render for ErrorResponse {
    fn render(&self) -> Markup {
        html! {
            div id="error" role="alert"
                ."z-10 bg-red-100 border border-red-400"
                ."text-red-700 p-4 h-14 -mb-14"
                ."rounded relative"
                _="on load wait 3s then transition my opacity to 0 then remove me"
            {
                strong class="font-bold" { "Holy smokes! " }
                span class="block sm:inline" { (self.message) }
                span class="absolute top-0 bottom-0 right-0 px-4 py-3"
                     _="on click remove closest #error"
                {
                    svg class="fill-current h-full w-6 text-red-500"
                        role="button"
                        xmlns="http://www.w3.org/2000/svg"
                        viewBox="0 0 20 20"
                    {
                        title { "Close" }
                        path d="M14.348 14.849a1.2 1.2 0 0 1-1.697 0L10 11.819l-2.651 3.029a1.2 1.2 0 1 1-1.697-1.697l2.758-3.15-2.759-3.152a1.2 1.2 0 1 1 1.697-1.697L10 8.183l2.651-3.031a1.2 1.2 0 1 1 1.697 1.697l-2.758 3.152 2.758 3.15a1.2 1.2 0 0 1 0 1.698z" {}
                    }
                }
            }
        }
    }
}
