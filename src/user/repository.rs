use diesel::BoolExpressionMethods;
use diesel::ExpressionMethods;
use diesel::PgConnection;
use diesel::QueryDsl;
use diesel::RunQueryDsl;
use diesel::SelectableHelper;
use diesel::r2d2::ConnectionManager;

use crate::schema::users;

use super::Nickname;
use super::Sub;
use super::model::NewUser;
use super::model::User;

pub trait UserRepository {
    fn insert(&self, user: &NewUser) -> super::Result<()>;

    fn find_by_sub(&self, sub: &Sub) -> super::Result<User>;

    fn search_by_nickname_excluding(
        &self,
        nickname: &Nickname,
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
    fn insert(&self, u: &NewUser) -> super::Result<()> {
        let mut conn = self.pool.get()?;

        let _ = diesel::insert_into(users::table)
            .values(u)
            .returning(users::sub)
            .get_result::<String>(&mut conn)?;

        Ok(())
    }

    fn find_by_sub(&self, s: &Sub) -> super::Result<User> {
        let mut conn = self.pool.get()?;

        let u = users::table
            .filter(users::sub.eq(s.as_str()))
            .limit(1)
            .select(User::as_select())
            .first(&mut conn)?;

        Ok(u)
    }

    fn search_by_nickname_excluding(
        &self,
        nickname: &Nickname,
        exclude: &Nickname,
    ) -> super::Result<Vec<User>> {
        let mut conn = self.pool.get()?;

        let users = users::table
            .filter(
                users::nickname
                    .eq(nickname.as_str())
                    .and(users::nickname.ne(exclude.as_str())),
            )
            .select(User::as_select())
            .get_results(&mut conn)?;

        Ok(users)
    }
}
