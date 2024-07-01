use std::fmt::{Display, Formatter};

use crate::{chat::model::ChatId, user::model::UserSub};

#[derive(Clone)]
pub enum CacheKey {
    UserInfo(UserSub),
    UsersOnline,
    Friends(UserSub),
    Chat(ChatId),
}

impl Display for CacheKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheKey::UserInfo(sub) => write!(f, "userinfo:{}", sub),
            CacheKey::UsersOnline => write!(f, "users:online"),
            CacheKey::Friends(sub) => write!(f, "friends:{}", sub),
            CacheKey::Chat(id) => write!(f, "chat:{}", id),
        }
    }
}

impl redis::ToRedisArgs for CacheKey {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        self.to_string().write_redis_args(out);
    }
}

#[cfg(test)]
mod tests {
    use crate::chat::model::ChatId;
    use crate::integration::model::CacheKey;
    use crate::user::model::UserSub;

    #[test]
    fn should_construct_user_info_cache_key() {
        let key = CacheKey::UserInfo(UserSub::from("valera"));
        assert_eq!(key.to_string(), "userinfo:valera");
    }

    #[test]
    fn should_construct_users_online_cache_key() {
        let key = CacheKey::UsersOnline;
        assert_eq!(key.to_string(), "users:online");
    }

    #[test]
    fn should_construct_friends_cache_key() {
        let key = CacheKey::Friends(UserSub::from("valera"));
        assert_eq!(key.to_string(), "friends:valera");
    }

    #[test]
    fn should_construct_chat_cache_key() {
        let chat_id = ChatId::parse_str("507f1f77bcf86cd799439011").unwrap();
        let key = CacheKey::Chat(chat_id);
        assert_eq!(key.to_string(), "chat:507f1f77bcf86cd799439011");
    }
}
