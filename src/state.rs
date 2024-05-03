use std::sync::Arc;

use crate::chat::repository::ChatRepository;
use crate::chat::service::ChatService;
use crate::integration::client;
use crate::message::repository::MessageRepository;
use crate::message::service::MessageService;
use crate::result::Result;
use crate::user::repository::UserRepository;
use crate::user::service::UserService;

#[derive(Clone)]
pub(crate) struct AppState {
    pub message_service: Arc<MessageService>,
    pub chat_service: Arc<ChatService>,
    pub user_service: Arc<UserService>,
}

impl AppState {
    pub async fn init() -> Result<Self> {
        let database = client::init_mongodb().await?;
        let _ = client::init_redis()?;
        let rabbitmq_con = client::init_rabbitmq().await?;
        let oidc_client = client::init_oidc_client().await?;

        Ok(Self {
            message_service: MessageService::new(MessageRepository::new(&database), rabbitmq_con),
            chat_service: ChatService::new(ChatRepository::new(&database)),
            user_service: UserService::new(UserRepository::new(&database), oidc_client),
        })
    }
}

// TODO: investigate
// impl FromRef<AppState> for MessageService {
//     fn from_ref(state: &AppState) -> Arc<Self> {
//         state.message_service.clone()
//     }
// }
