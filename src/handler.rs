use crate::markup::{self, Wrappable};
use axum::{Extension, extract::State};

use crate::{talk, user::model::UserInfo};

pub async fn home(
    _user_info: Extension<UserInfo>,
    _talk_service: State<talk::Service>,
) -> crate::Result<Wrappable> {
    // TODO:
    // let talks = talk_service.find_all(&user_info).await?;

    // first shown component will be talk page
    Ok(Wrappable::new(markup::Tabs {}).with_sse())
}
