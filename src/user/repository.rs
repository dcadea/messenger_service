use diesel::BoolExpressionMethods;
use diesel::ExpressionMethods;
use diesel::PgConnection;
use diesel::QueryDsl;
use diesel::RunQueryDsl;
use diesel::SelectableHelper;
use diesel::TextExpressionMethods;
use diesel::r2d2::ConnectionManager;

use crate::schema::users;

use super::Id;
use super::Nickname;
use super::Sub;
use super::model::NewUser;
use super::model::User;

pub trait UserRepository {
    fn insert(&self, user: &NewUser) -> super::Result<()>;

    fn find_by_id(&self, id: &Id) -> super::Result<User>;

    fn find_by_sub(&self, sub: &Sub) -> super::Result<User>;

    fn exists(&self, id: &Id) -> super::Result<bool>;

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
    fn insert(&self, user: &NewUser) -> super::Result<()> {
        let mut conn = self.pool.get()?;

        let _ = diesel::insert_into(users::table)
            .values(user)
            .returning(users::sub)
            .get_result::<String>(&mut conn)?;

        Ok(())
    }

    fn find_by_id(&self, id: &Id) -> super::Result<User> {
        let mut conn = self.pool.get()?;

        let u = users::table
            .find(id.0)
            .select(User::as_select())
            .first(&mut conn)?;

        Ok(u)
    }

    fn find_by_sub(&self, sub: &Sub) -> super::Result<User> {
        let mut conn = self.pool.get()?;

        let u = users::table
            .filter(users::sub.eq(sub.as_str()))
            .select(User::as_select())
            .first(&mut conn)?;

        Ok(u)
    }

    fn exists(&self, id: &Id) -> super::Result<bool> {
        let mut conn = self.pool.get()?;

        let count = users::table
            .find(id.0)
            .count()
            .get_result::<i64>(&mut conn)?;

        Ok(count > 0)
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
                    .like(nickname.as_str())
                    .and(users::nickname.ne(exclude.as_str())),
            )
            .select(User::as_select())
            .get_results(&mut conn)?;

        Ok(users)
    }
}
