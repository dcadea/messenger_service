use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::auth;

use super::{model::UserInfo, service::UserService};

pub async fn cache_user_friends(
    user_service: State<UserService>,
    request: Request,
    next: Next,
) -> crate::Result<Response> {
    let user_info = request
        .extensions()
        .get::<UserInfo>()
        .ok_or(auth::Error::Unauthorized)?;

    user_service.cache_friends(&user_info.sub).await?;

    let response = next.run(request).await;
    Ok(response)
}
