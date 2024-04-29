use crate::chat::service::ChatService;
use std::sync::Arc;

use crate::message::service::MessageService;
use crate::user::service::UserService;

#[derive(Clone)]
pub struct AppState {
    pub message_service: Arc<MessageService>,
    pub chat_service: Arc<ChatService>,
    pub user_service: Arc<UserService>,
}

// TODO: investigate
// impl FromRef<AppState> for MessageService {
//     fn from_ref(state: &AppState) -> Arc<Self> {
//         state.message_service.clone()
//     }
// }
