use axum::{extract::State, response::IntoResponse, Form};
use serde::Deserialize;

use super::{markup, service::UserService};

#[derive(Deserialize)]
pub struct FindParams {
    nickname: String,
}

pub async fn search(
    user_service: State<UserService>,
    params: Form<FindParams>,
) -> impl IntoResponse {
    let users = match user_service.search_user_info(&params.nickname).await {
        Ok(users) => users,
        Err(err) => return crate::error::Error::from(err).into_response(),
    };

    markup::search_result(&users).into_response()
}
