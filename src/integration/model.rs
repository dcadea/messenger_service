use crate::user::model::UserSub;

pub enum CacheKey {
    UserInfo(UserSub),
    UsersOnline,
}

impl CacheKey {
    pub fn to_string(&self) -> String {
        match self {
            CacheKey::UserInfo(sub) => format!("userinfo:{}", sub),
            CacheKey::UsersOnline => "users:online".to_string(),
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
