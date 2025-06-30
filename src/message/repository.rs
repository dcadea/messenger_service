use chrono::{DateTime, Utc};
use diesel::{
    BoolExpressionMethods, ExpressionMethods, OptionalExtension, PgConnection, QueryDsl,
    RunQueryDsl, SelectableHelper, delete, insert_into, r2d2::ConnectionManager, update,
};

use crate::{message, user};

use super::model::{Message, NewMessage};
use crate::{
    schema::messages::dsl::{content, created_at, id, messages, owner, seen, talk_id},
    talk,
};

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
        let mut conn = self.pool.get()?;

        insert_into(messages)
            .values(msg)
            .returning(Message::as_select())
            .get_result(&mut conn)
            .map_err(super::Error::from)
    }

    fn insert_many(&self, msgs: &[NewMessage]) -> super::Result<Vec<Message>> {
        let mut conn = self.pool.get()?;

        insert_into(messages)
            .values(msgs)
            .returning(Message::as_select())
            .get_results(&mut conn)
            .map_err(super::Error::from)
    }

    fn find_by_id(&self, o: &user::Id, m_id: &message::Id) -> super::Result<Message> {
        let mut conn = self.pool.get()?;

        messages
            .filter(id.eq(m_id).and(owner.eq(o)))
            .select(Message::as_select())
            .first(&mut conn)
            .map_err(super::Error::from)
    }

    fn find_by_talk_id(&self, t_id: &talk::Id) -> super::Result<Vec<Message>> {
        let mut conn = self.pool.get()?;

        messages
            .filter(talk_id.eq(t_id))
            .select(Message::as_select())
            .get_results(&mut conn)
            .map_err(super::Error::from)
    }

    fn find_by_talk_id_limited(&self, t_id: &talk::Id, limit: i64) -> super::Result<Vec<Message>> {
        let mut conn = self.pool.get()?;

        messages
            .filter(talk_id.eq(t_id))
            .limit(limit)
            .order(created_at.desc())
            .select(Message::as_select())
            .get_results(&mut conn)
            .map_err(super::Error::from)
    }

    fn find_by_talk_id_before(
        &self,
        t_id: &talk::Id,
        before: DateTime<Utc>,
    ) -> super::Result<Vec<Message>> {
        let mut conn = self.pool.get()?;

        messages
            .filter(talk_id.eq(t_id).and(created_at.lt(before)))
            .order(created_at.desc())
            .select(Message::as_select())
            .get_results(&mut conn)
            .map_err(super::Error::from)
    }

    fn find_by_talk_id_limited_before(
        &self,
        t_id: &talk::Id,
        limit: i64,
        before: DateTime<Utc>,
    ) -> super::Result<Vec<Message>> {
        let mut conn = self.pool.get()?;

        messages
            .filter(talk_id.eq(t_id).and(created_at.lt(before)))
            .limit(limit)
            .order(created_at.desc())
            .select(Message::as_select())
            .get_results(&mut conn)
            .map_err(super::Error::from)
    }

    fn update(
        &self,
        o: &user::Id,
        m_id: &message::Id,
        new_content: &str,
    ) -> super::Result<Option<Message>> {
        let mut conn = self.pool.get()?;

        let updated_msg = update(messages.filter(id.eq(m_id).and(owner.eq(o))))
            .set(content.eq(new_content))
            .returning(Message::as_returning())
            .get_result(&mut conn)
            .optional()?;

        Ok(updated_msg)
    }

    fn delete(&self, o: &user::Id, m_id: &message::Id) -> super::Result<Option<Message>> {
        let mut conn = self.pool.get()?;

        let deleted_msg = delete(messages.filter(id.eq(m_id).and(owner.eq(o))))
            .returning(Message::as_returning())
            .get_result(&mut conn)
            .optional()?;

        Ok(deleted_msg)
    }

    fn mark_as_seen(&self, ids: &[message::Id]) -> super::Result<usize> {
        let mut conn = self.pool.get()?;

        let modified_count = update(messages.filter(id.eq_any(ids)))
            .set(seen.eq(true))
            .execute(&mut conn)?;

        Ok(modified_count)
    }
}
