use diesel::{
    BoolExpressionMethods, ExpressionMethods, OptionalExtension, PgConnection, QueryDsl,
    RunQueryDsl, SelectableHelper, delete, insert_into, r2d2::ConnectionManager, update,
};

use crate::schema::contacts::dsl::{contacts, id, initiator, status, user_id_1, user_id_2};

use crate::contact::{self};
use crate::user;

use super::{
    Status,
    model::{Contact, NewContact},
};

pub trait ContactRepository {
    fn find(&self, u1: &user::Id, u2: &user::Id) -> super::Result<Option<Contact>>;

    fn find_by_id(&self, c_id: &contact::Id) -> super::Result<Option<Contact>>;

    fn find_by_user_id(&self, user_id: &user::Id) -> super::Result<Vec<Contact>>;

    fn add(&self, c: &NewContact) -> super::Result<()>;

    fn update_status(&self, c_id: &contact::Id, status: &Status) -> super::Result<bool>;

    fn delete(&self, me: &user::Id, you: &user::Id) -> super::Result<bool>;

    fn exists(&self, me: &user::Id, you: &user::Id) -> super::Result<bool>;
}

pub struct PgContactRepository {
    pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}

impl PgContactRepository {
    pub const fn new(pool: r2d2::Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }
}

impl ContactRepository for PgContactRepository {
    fn find(&self, me: &user::Id, you: &user::Id) -> super::Result<Option<Contact>> {
        let mut conn = self.pool.get()?;

        contacts
            .filter(
                (user_id_1.eq(me).and(user_id_2.eq(you)))
                    .or(user_id_1.eq(you).and(user_id_2.eq(me))),
            )
            .first::<Contact>(&mut conn)
            .optional()
            .map_err(super::Error::from)
    }

    fn find_by_id(&self, c_id: &contact::Id) -> super::Result<Option<Contact>> {
        let mut conn = self.pool.get()?;

        contacts
            .find(c_id)
            .first::<Contact>(&mut conn)
            .optional()
            .map_err(super::Error::from)
    }

    fn find_by_user_id(&self, user_id: &user::Id) -> super::Result<Vec<Contact>> {
        let mut conn = self.pool.get()?;

        contacts
            .filter(user_id_1.eq(user_id).or(user_id_2.eq(user_id)))
            .select(Contact::as_select())
            .get_results(&mut conn)
            .map_err(super::Error::from)
    }

    fn add(&self, c: &NewContact) -> super::Result<()> {
        assert_ne!(c.user_id_1(), c.user_id_2());

        let mut conn = self.pool.get()?;

        insert_into(contacts).values(c).execute(&mut conn)?;

        Ok(())
    }

    fn update_status(&self, c_id: &contact::Id, s: &Status) -> super::Result<bool> {
        let mut conn = self.pool.get()?;

        let i = match s {
            Status::Accepted | Status::Rejected => None,
            Status::Pending { initiator: i } | Status::Blocked { initiator: i } => Some(i),
        };

        let modified_count = update(contacts)
            .filter(id.eq(c_id))
            .set((status.eq(s.as_str()), initiator.eq(i)))
            .execute(&mut conn)?;

        Ok(modified_count > 0)
    }

    fn delete(&self, me: &user::Id, you: &user::Id) -> super::Result<bool> {
        let mut conn = self.pool.get()?;

        let deleted_count = delete(contacts)
            .filter(
                (user_id_1.eq(me).and(user_id_2.eq(you)))
                    .or(user_id_1.eq(you).and(user_id_2.eq(me))),
            )
            .execute(&mut conn)?;

        Ok(deleted_count > 0)
    }

    fn exists(&self, me: &user::Id, you: &user::Id) -> super::Result<bool> {
        let mut conn = self.pool.get()?;
        let count: i64 = contacts
            .filter(
                (user_id_1.eq(me).and(user_id_2.eq(you)))
                    .or(user_id_1.eq(you).and(user_id_2.eq(me))),
            )
            .count()
            .get_result(&mut conn)?;

        Ok(count > 0)
    }
}
