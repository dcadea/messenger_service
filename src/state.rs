use std::sync::Arc;

use crate::message::repository::MessageRepository;
use crate::message::service::MessageService;
use crate::user::repository::UserRepository;

#[derive(Clone)]
pub struct AppState {
    pub message_service: Arc<MessageService>,

    pub user_repository: Arc<UserRepository>,
    pub message_repository: Arc<MessageRepository>,
}

// TODO: investigate
// impl FromRef<AppState> for MessageService {
//     fn from_ref(state: &AppState) -> Arc<Self> {
//         state.message_service.clone()
//     }
// }
