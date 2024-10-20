use axum::{extract::State, Form};
use maud::{html, Markup};
use serde::Deserialize;

use super::{markup, service::UserService};

#[derive(Deserialize)]
pub struct FindParams {
    nickname: String,
}

pub async fn search(
    user_service: State<UserService>,
    params: Form<FindParams>,
) -> crate::Result<Markup> {
    if params.nickname.is_empty() {
        return Ok(html! {(messenger_service::markup::EMPTY)});
    }

    let users = user_service.search_user_info(&params.nickname).await?;

    Ok(markup::search_result(&users))
}
