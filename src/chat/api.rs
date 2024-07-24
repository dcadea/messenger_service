use axum::extract::{Path, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};

use crate::model::AppEndpoints;
use crate::result::Result;
use crate::state::AppState;
use crate::user::model::UserInfo;

use super::model::{ChatDto, ChatId, ChatRequest};
use super::service::ChatService;

pub fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/chats", get(find_handler))
        .route("/chats/:id", get(find_by_id_handler))
        .route("/chats", post(create_handler))
        .with_state(state)
}

async fn find_handler(
    user_info: Extension<UserInfo>,
    chat_service: State<ChatService>,
) -> Result<Json<Vec<ChatDto>>> {
    let result = chat_service.find_all(&user_info).await?;
    Ok(Json(result))
}

async fn find_by_id_handler(
    user_info: Extension<UserInfo>,
    chat_service: State<ChatService>,
    Path(id): Path<ChatId>,
) -> Result<Json<ChatDto>> {
    let result = chat_service.find_by_id(&id, &user_info).await?;
    Ok(Json(result))
}

async fn create_handler(
    user_info: Extension<UserInfo>,
    chat_service: State<ChatService>,
    app_endpoints: State<AppEndpoints>,
    Json(chat_request): Json<ChatRequest>,
) -> Result<impl IntoResponse> {
    let base_url = app_endpoints.api();
    let result = chat_service.create(&chat_request, &user_info).await?;
    let location = format!("{base_url}/chats/{}", &result.id);

    let mut response = Json(result).into_response();
    *response.status_mut() = StatusCode::CREATED;
    response
        .headers_mut()
        .insert(header::LOCATION, HeaderValue::from_str(&location)?);

    Ok(response)
}
