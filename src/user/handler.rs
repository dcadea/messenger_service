use axum::{extract::State, Extension, Form};
use maud::{html, Markup};
use serde::Deserialize;

use super::{markup, model::UserInfo, service::UserService};

#[derive(Deserialize)]
pub struct FindParams {
    nickname: String,
}

pub async fn search(
    user_info: Extension<UserInfo>,
    user_service: State<UserService>,
    params: Form<FindParams>,
) -> crate::Result<Markup> {
    if params.nickname.is_empty() {
        return Ok(html! {(messenger_service::markup::EMPTY)});
    }

    let users = user_service
        .search_user_info(&params.nickname, &user_info.nickname)
        .await?;

    let friends = user_service
        .find_cached_friends(&user_info.sub)
        .await
        .unwrap_or(user_info.friends.clone());

    Ok(markup::search_result(&friends, &users))
}
