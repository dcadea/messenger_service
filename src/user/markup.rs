use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use axum_extra::extract::Query;
use maud::{html, Markup, Render};

use crate::error::Error;
use crate::state::AppState;

use super::model::UserParams;
use super::service::UserService;

pub fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/users", get(find_handler))
        .with_state(state)
}

async fn find_handler(
    Query(params): Query<UserParams>,
    user_service: State<UserService>,
) -> impl IntoResponse {
    match params.sub {
        Some(sub) => match user_service.find_user_info(&sub).await {
            Ok(user_info) => Json(user_info).into_response(),
            Err(err) => Error::from(err).into_response(),
        },
        None => match params.nickname {
            Some(nickname) => match user_service.search_user_info(&nickname).await {
                Ok(user_infos) => Json(user_infos).into_response(),
                Err(err) => Error::from(err).into_response(),
            },
            None => Error::QueryParamRequired("sub or nickname".to_owned()).into_response(),
        },
    }
}

pub(crate) struct UserHeader<'a> {
    pub name: &'a str,
    pub picture: &'a str,
}

impl Render for UserHeader<'_> {
    fn render(&self) -> Markup {
        html! {
            header."flex justify-between items-center mb-4" {
                img."w-12 h-12 rounded-full mr-2"
                    src=(self.picture)
                    alt="User avatar" {}
                h2.text-2xl {(self.name)}
                a."bg-red-500 hover:bg-red-700 text-white font-bold py-2 px-4 rounded"
                    href="/logout" { "Logout" }
            }
        }
    }
}
