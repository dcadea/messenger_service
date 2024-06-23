use crate::user::model::UserSub;

pub enum CacheKey {
    UserInfo { sub: UserSub, ttl: u64 },
    UsersOnline,
}

impl CacheKey {
    pub fn to_string(&self) -> String {
        match self {
            CacheKey::UserInfo { sub, .. } => format!("userinfo:{}", sub),
            CacheKey::UsersOnline => "users:online".to_string(),
        }
    }

    pub fn ttl(&self) -> u64 {
        match self {
            CacheKey::UserInfo { ttl, .. } => *ttl,
            CacheKey::UsersOnline => 0,
        }
    }
}
