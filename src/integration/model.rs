use std::fmt::{Display, Formatter};
use crate::user::model::UserSub;

pub enum CacheKey {
    UserInfo(UserSub),
    UsersOnline,
    Friends(UserSub),
}

impl Display for CacheKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheKey::UserInfo(sub) => write!(f, "userinfo:{}", sub),
            CacheKey::UsersOnline => write!(f, "users:online"),
            CacheKey::Friends(sub) => write!(f, "friends:{}", sub),
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
