use chrono::NaiveDateTime;
use diesel::{
    BoolExpressionMethods, ExpressionMethods, OptionalExtension, PgConnection, QueryDsl,
    RunQueryDsl, SelectableHelper, delete, insert_into, r2d2::ConnectionManager, update,
};

use crate::{message, user};

use super::model::{Message, NewMessage};
use crate::{schema::messages::dsl::*, talk};

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
        before: NaiveDateTime,
    ) -> super::Result<Vec<Message>>;

    fn find_by_talk_id_limited_before(
        &self,
        talk_id: &talk::Id,
        limit: i64,
        before: NaiveDateTime,
    ) -> super::Result<Vec<Message>>;

    fn find_most_recent(&self, talk_id: &talk::Id) -> super::Result<Option<Message>>;

    fn update(&self, owner: &user::Id, id: &message::Id, new_content: &str) -> super::Result<bool>;

    fn delete(&self, owner: &user::Id, id: &message::Id) -> super::Result<bool>;

    fn mark_as_seen(&self, ids: &[message::Id]) -> super::Result<usize>;
}

pub struct PgMessageRepository {
    pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}

impl PgMessageRepository {
    pub fn new(pool: r2d2::Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }
}

impl MessageRepository for PgMessageRepository {
    fn insert(&self, msg: &NewMessage) -> super::Result<Message> {
        let mut conn = self.pool.get()?;

        let m = insert_into(messages)
            .values(msg)
            .returning(Message::as_select())
            .get_result(&mut conn)?;

        Ok(m)
    }

    fn insert_many(&self, msgs: &[NewMessage]) -> super::Result<Vec<Message>> {
        let mut conn = self.pool.get()?;

        let msgs = insert_into(messages)
            .values(msgs)
            .returning(Message::as_select())
            .get_results(&mut conn)?;

        Ok(msgs)
    }

    fn find_by_id(&self, o: &user::Id, m_id: &message::Id) -> super::Result<Message> {
        let mut conn = self.pool.get()?;

        let m = messages
            .filter(id.eq(m_id).and(owner.eq(o)))
            .select(Message::as_select())
            .first(&mut conn)?;

        Ok(m)
    }

    fn find_by_talk_id(&self, t_id: &talk::Id) -> super::Result<Vec<Message>> {
        let mut conn = self.pool.get()?;

        let msgs = messages
            .filter(talk_id.eq(t_id))
            .select(Message::as_select())
            .get_results(&mut conn)?;

        Ok(msgs)
    }

    fn find_by_talk_id_limited(&self, t_id: &talk::Id, limit: i64) -> super::Result<Vec<Message>> {
        let mut conn = self.pool.get()?;

        let msgs = messages
            .filter(talk_id.eq(t_id))
            .limit(limit)
            .order(created_at.desc())
            .select(Message::as_select())
            .get_results(&mut conn)?;

        Ok(msgs)
    }

    fn find_by_talk_id_before(
        &self,
        t_id: &talk::Id,
        before: NaiveDateTime,
    ) -> super::Result<Vec<Message>> {
        let mut conn = self.pool.get()?;

        let msgs = messages
            .filter(talk_id.eq(t_id).and(created_at.lt(before)))
            .order(created_at.desc())
            .select(Message::as_select())
            .get_results(&mut conn)?;

        Ok(msgs)
    }

    fn find_by_talk_id_limited_before(
        &self,
        t_id: &talk::Id,
        limit: i64,
        before: NaiveDateTime,
    ) -> super::Result<Vec<Message>> {
        let mut conn = self.pool.get()?;

        let msgs = messages
            .filter(talk_id.eq(t_id).and(created_at.lt(before)))
            .limit(limit)
            .order(created_at.desc())
            .select(Message::as_select())
            .get_results(&mut conn)?;

        Ok(msgs)
    }

    fn find_most_recent(&self, t_id: &talk::Id) -> super::Result<Option<Message>> {
        let mut conn = self.pool.get()?;

        let msg = messages
            .filter(talk_id.eq(t_id))
            .limit(1)
            .order(created_at.desc())
            .select(Message::as_select())
            .first(&mut conn)
            .optional()?;

        Ok(msg)
    }

    fn update(&self, o: &user::Id, m_id: &message::Id, new_content: &str) -> super::Result<bool> {
        let mut conn = self.pool.get()?;

        let res = update(messages.filter(id.eq(m_id).and(owner.eq(o))))
            .set(content.eq(new_content))
            .execute(&mut conn)?;

        Ok(res > 0)
    }

    fn delete(&self, o: &user::Id, m_id: &message::Id) -> super::Result<bool> {
        let mut conn = self.pool.get()?;

        let deleted_count =
            delete(messages.filter(id.eq(m_id).and(owner.eq(o)))).execute(&mut conn)?;

        Ok(deleted_count > 0)
    }

    fn mark_as_seen(&self, ids: &[message::Id]) -> super::Result<usize> {
        let mut conn = self.pool.get()?;

        let modified_count = update(messages.filter(id.eq_any(ids)))
            .set(seen.eq(true))
            .execute(&mut conn)?;

        Ok(modified_count)
    }
}
