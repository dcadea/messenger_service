use std::sync::Arc;

use crate::chat::repository::ChatRepository;
use crate::chat::service::ChatService;
use crate::integration;
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

    pub http: Arc<reqwest::Client>,
}

impl AppState {
    pub async fn init(config: &integration::Config) -> Result<Self> {
        let database = integration::init_mongodb(config).await?;
        let _ = integration::init_redis(config)?;
        let rabbitmq_con = integration::init_rabbitmq(config).await?;
        let http = integration::init_http_client()?;

        Ok(Self {
            message_service: MessageService::new(MessageRepository::new(&database), rabbitmq_con),
            chat_service: ChatService::new(ChatRepository::new(&database)),
            user_service: UserService::new(UserRepository::new(&database)),
            http
        })
    }
}

// TODO: investigate
// impl FromRef<AppState> for MessageService {
//     fn from_ref(state: &AppState) -> Arc<Self> {
//         state.message_service.clone()
//     }
// }
