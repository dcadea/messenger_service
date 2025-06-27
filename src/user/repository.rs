use diesel::BoolExpressionMethods;
use diesel::CombineDsl;
use diesel::ExpressionMethods;
use diesel::OptionalExtension;
use diesel::PgConnection;
use diesel::QueryDsl;
use diesel::RunQueryDsl;
use diesel::SelectableHelper;
use diesel::TextExpressionMethods;
use diesel::insert_into;
use diesel::r2d2::ConnectionManager;
use uuid::Uuid;

use crate::schema::users::dsl::*;
use crate::talk;

use super::Nickname;
use super::Sub;
use super::model::NewUser;
use super::model::User;
use crate::user;

pub trait UserRepository {
    fn create(&self, u: &NewUser) -> super::Result<user::Id>;

    fn find_by_id(&self, u_id: &user::Id) -> super::Result<User>;

    fn find_by_sub(&self, s: &Sub) -> super::Result<Option<User>>;

    fn find_by_talk_id(&self, talk_id: &talk::Id) -> super::Result<Vec<user::Id>>;

    fn exists(&self, u_id: &user::Id) -> super::Result<bool>;

    fn find_by_nickname_like_and_excluding(
        &self,
        n: &Nickname,
        exclude: &Nickname,
    ) -> super::Result<Vec<User>>;
}

pub struct PgUserRepository {
    pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}

impl PgUserRepository {
    pub fn new(pool: r2d2::Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }
}

impl UserRepository for PgUserRepository {
    fn create(&self, u: &NewUser) -> super::Result<user::Id> {
        let mut conn = self.pool.get()?;

        insert_into(users)
            .values(u)
            .returning(id)
            .get_result::<Uuid>(&mut conn)
            .map(|i| user::Id(i))
            .map_err(super::Error::from)
    }

    fn find_by_id(&self, u_id: &user::Id) -> super::Result<User> {
        let mut conn = self.pool.get()?;

        let u = users
            .find(u_id)
            .select(User::as_select())
            .first(&mut conn)?;

        Ok(u)
    }

    fn find_by_sub(&self, s: &Sub) -> super::Result<Option<User>> {
        let mut conn = self.pool.get()?;

        let u = users
            .filter(sub.eq(s.as_str()))
            .select(User::as_select())
            .get_result(&mut conn)
            .optional()?;

        Ok(u)
    }

    fn find_by_talk_id(&self, t_id: &talk::Id) -> super::Result<Vec<user::Id>> {
        use crate::schema::chats_users::dsl as cu;
        use crate::schema::groups_users::dsl as gu;

        let mut conn = self.pool.get()?;

        gu::groups_users
            .filter(gu::group_id.eq(t_id))
            .select(gu::user_id)
            .union(
                cu::chats_users
                    .filter(cu::chat_id.eq(t_id))
                    .select(cu::user_id),
            )
            .load(&mut conn)
            .map_err(super::Error::from)
    }

    fn exists(&self, u_id: &user::Id) -> super::Result<bool> {
        let mut conn = self.pool.get()?;

        let count = users.find(u_id).count().get_result::<i64>(&mut conn)?;

        Ok(count > 0)
    }

    fn find_by_nickname_like_and_excluding(
        &self,
        n: &Nickname,
        exclude: &Nickname,
    ) -> super::Result<Vec<User>> {
        let mut conn = self.pool.get()?;

        users
            .filter(
                nickname
                    .like(format!("%{}%", n.as_str()))
                    .and(nickname.ne(exclude.as_str())),
            )
            .select(User::as_select())
            .get_results(&mut conn)
            .map_err(super::Error::from)
    }
}
