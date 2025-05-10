use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use log::error;
use maud::{Markup, Render, html};
use serde::Serialize;

use crate::{auth, contact, event, message, talk, user};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Query parameter '{0}' is required")]
    QueryParamRequired(String),

    #[error(transparent)]
    _Auth(#[from] auth::Error),
    #[error(transparent)]
    _Contact(#[from] contact::Error),
    #[error(transparent)]
    _Talk(#[from] talk::Error),
    #[error(transparent)]
    _Event(#[from] event::Error),
    #[error(transparent)]
    _Message(#[from] message::Error),
    #[error(transparent)]
    _User(#[from] user::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let error_message = {
            let mut error_message = self.to_string();
            error!("{error_message:?}");

            let status = StatusCode::from(self);
            if status.is_server_error() {
                "Internal server error".clone_into(&mut error_message);
            }
            error_message
        };

        let response = {
            let mut r = ErrorResponse { error_message }.render().into_response();
            r.headers_mut()
                .insert("HX-Retarget", HeaderValue::from_static("#errors"));

            r
        };

        response
    }
}

impl From<Error> for StatusCode {
    fn from(e: Error) -> Self {
        match e {
            Error::QueryParamRequired(_) => Self::BAD_REQUEST,
            Error::_Auth(a) => a.into(),
            Error::_Contact(c) => c.into(),
            Error::_Talk(t) => t.into(),
            Error::_Event(e) => e.into(),
            Error::_Message(m) => m.into(),
            Error::_User(u) => u.into(),
        }
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error_message: String,
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
                span class="block sm:inline" { (self.error_message) }
                span class="absolute top-0 bottom-0 right-0 px-4 py-3"
                     _="on click remove closest #error"
                {
                    svg class="fill-current h-full w-6 text-red-500 cursor-pointer"
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
