use chrono::{DateTime, Utc};
use diesel::{
    BoolExpressionMethods, ExpressionMethods, OptionalExtension, PgConnection, QueryDsl,
    RunQueryDsl, SelectableHelper, delete, insert_into, r2d2::ConnectionManager, update,
};

use crate::{message, user};

use super::model::{Message, NewMessage};
use crate::talk;

pub trait MessageRepository {
    fn insert(&self, new_message: &NewMessage) -> super::Result<Message>;

    fn insert_many(&self, new_messages: &[NewMessage]) -> super::Result<Vec<Message>>;

    fn find_by_id(&self, owner: &user::Id, id: &message::Id) -> super::Result<Message>;

    fn find_by_talk_id(&self, talk_id: &talk::Id) -> super::Result<Vec<Message>>;

    fn find_by_talk_id_limited(
        &self,
        talk_id: &talk::Id,
        limit: i64,
    ) -> super::Result<Vec<Message>>;

    fn find_by_talk_id_before(
        &self,
        talk_id: &talk::Id,
        before: DateTime<Utc>,
    ) -> super::Result<Vec<Message>>;

    fn find_by_talk_id_limited_before(
        &self,
        talk_id: &talk::Id,
        limit: i64,
        before: DateTime<Utc>,
    ) -> super::Result<Vec<Message>>;

    fn update(
        &self,
        owner: &user::Id,
        id: &message::Id,
        new_content: &str,
    ) -> super::Result<Option<Message>>;

    fn delete(&self, owner: &user::Id, id: &message::Id) -> super::Result<Option<Message>>;

    fn mark_as_seen(&self, ids: &[message::Id]) -> super::Result<usize>;
}

pub struct PgMessageRepository {
    pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}

impl PgMessageRepository {
    pub const fn new(pool: r2d2::Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }
}

impl MessageRepository for PgMessageRepository {
    fn insert(&self, msg: &NewMessage) -> super::Result<Message> {
        use crate::schema::messages::dsl::messages;

        let mut conn = self.pool.get()?;

        insert_into(messages)
            .values(msg)
            .returning(Message::as_select())
            .get_result(&mut conn)
            .map_err(super::Error::from)
    }

    fn insert_many(&self, msgs: &[NewMessage]) -> super::Result<Vec<Message>> {
        use crate::schema::messages::dsl::messages;

        let mut conn = self.pool.get()?;

        insert_into(messages)
            .values(msgs)
            .returning(Message::as_select())
            .get_results(&mut conn)
            .map_err(super::Error::from)
    }

    fn find_by_id(&self, owner: &user::Id, id: &message::Id) -> super::Result<Message> {
        use crate::schema::messages::dsl as m;

        let mut conn = self.pool.get()?;

        m::messages
            .filter(m::id.eq(id).and(m::owner.eq(owner)))
            .select(Message::as_select())
            .first(&mut conn)
            .map_err(super::Error::from)
    }

    fn find_by_talk_id(&self, talk_id: &talk::Id) -> super::Result<Vec<Message>> {
        use crate::schema::messages::dsl as m;

        let mut conn = self.pool.get()?;

        m::messages
            .filter(m::talk_id.eq(talk_id))
            .select(Message::as_select())
            .get_results(&mut conn)
            .map_err(super::Error::from)
    }

    fn find_by_talk_id_limited(
        &self,
        talk_id: &talk::Id,
        limit: i64,
    ) -> super::Result<Vec<Message>> {
        use crate::schema::messages::dsl as m;

        let mut conn = self.pool.get()?;

        m::messages
            .filter(m::talk_id.eq(talk_id))
            .limit(limit)
            .order(m::created_at.desc())
            .select(Message::as_select())
            .get_results(&mut conn)
            .map_err(super::Error::from)
    }

    fn find_by_talk_id_before(
        &self,
        talk_id: &talk::Id,
        before: DateTime<Utc>,
    ) -> super::Result<Vec<Message>> {
        use crate::schema::messages::dsl as m;

        let mut conn = self.pool.get()?;

        m::messages
            .filter(m::talk_id.eq(talk_id).and(m::created_at.lt(before)))
            .order(m::created_at.desc())
            .select(Message::as_select())
            .get_results(&mut conn)
            .map_err(super::Error::from)
    }

    fn find_by_talk_id_limited_before(
        &self,
        talk_id: &talk::Id,
        limit: i64,
        before: DateTime<Utc>,
    ) -> super::Result<Vec<Message>> {
        use crate::schema::messages::dsl as m;

        let mut conn = self.pool.get()?;

        m::messages
            .filter(m::talk_id.eq(talk_id).and(m::created_at.lt(before)))
            .limit(limit)
            .order(m::created_at.desc())
            .select(Message::as_select())
            .get_results(&mut conn)
            .map_err(super::Error::from)
    }

    fn update(
        &self,
        owner: &user::Id,
        id: &message::Id,
        new_content: &str,
    ) -> super::Result<Option<Message>> {
        use crate::schema::messages::dsl as m;

        let mut conn = self.pool.get()?;

        let updated_msg = update(m::messages.filter(m::id.eq(id).and(m::owner.eq(owner))))
            .set(m::content.eq(new_content))
            .returning(Message::as_returning())
            .get_result(&mut conn)
            .optional()?;

        Ok(updated_msg)
    }

    fn delete(&self, owner: &user::Id, id: &message::Id) -> super::Result<Option<Message>> {
        use crate::schema::messages::dsl as m;

        let mut conn = self.pool.get()?;

        let deleted_msg = delete(m::messages.filter(m::id.eq(id).and(m::owner.eq(owner))))
            .returning(Message::as_returning())
            .get_result(&mut conn)
            .optional()?;

        Ok(deleted_msg)
    }

    fn mark_as_seen(&self, ids: &[message::Id]) -> super::Result<usize> {
        use crate::schema::messages::dsl as m;

        let mut conn = self.pool.get()?;

        let modified_count = update(m::messages.filter(m::id.eq_any(ids)))
            .set(m::seen.eq(true))
            .execute(&mut conn)?;

        Ok(modified_count)
    }
}
