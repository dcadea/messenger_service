use axum::{extract::State, Form};
use maud::Markup;
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
    let users = user_service.search_user_info(&params.nickname).await?;

    Ok(markup::search_result(&users))
}
