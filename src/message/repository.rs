use chrono::NaiveDateTime;
use diesel::{
    BoolExpressionMethods, ExpressionMethods, OptionalExtension, PgConnection, QueryDsl,
    RunQueryDsl, SelectableHelper, r2d2::ConnectionManager,
};
use uuid::Uuid;

use super::{
    Id,
    model::{Message, NewMessage},
};
use crate::{schema::messages, talk};

pub trait MessageRepository {
    fn insert(&self, msg: &NewMessage) -> super::Result<Message>;

    fn insert_many(&self, msgs: &[NewMessage]) -> super::Result<Vec<Message>>;

    // TODO: use super::Id
    fn find_by_id(&self, id: &Id) -> super::Result<Message>;

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

    fn update(&self, id: &Id, content: &str) -> super::Result<bool>;

    fn delete(&self, id: &Id) -> super::Result<bool>;

    fn delete_by_talk_id(&self, talk_id: &talk::Id) -> super::Result<usize>;

    fn mark_as_seen(&self, ids: &[Id]) -> super::Result<usize>;
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

        let m = diesel::insert_into(messages::table)
            .values(msg)
            .returning(Message::as_select())
            .get_result(&mut conn)?;

        Ok(m)
    }

    fn insert_many(&self, msgs: &[NewMessage]) -> super::Result<Vec<Message>> {
        let mut conn = self.pool.get()?;

        let msgs = diesel::insert_into(messages::table)
            .values(msgs)
            .returning(Message::as_select())
            .get_results(&mut conn)?;

        Ok(msgs)
    }

    fn find_by_id(&self, id: &Id) -> super::Result<Message> {
        let mut conn = self.pool.get()?;

        let m = messages::table
            .find(id.0)
            .select(Message::as_select())
            .first(&mut conn)?;

        Ok(m)
    }

    fn find_by_talk_id(&self, talk_id: &talk::Id) -> super::Result<Vec<Message>> {
        let mut conn = self.pool.get()?;

        let msgs = messages::table
            .filter(messages::talk_id.eq(talk_id.0))
            .select(Message::as_select())
            .get_results(&mut conn)?;

        Ok(msgs)
    }

    fn find_by_talk_id_limited(
        &self,
        talk_id: &talk::Id,
        limit: i64,
    ) -> super::Result<Vec<Message>> {
        let mut conn = self.pool.get()?;

        let msgs = messages::table
            .filter(messages::talk_id.eq(talk_id.0))
            .limit(limit)
            .order(messages::created_at.desc())
            .select(Message::as_select())
            .get_results(&mut conn)?;

        Ok(msgs)
    }

    fn find_by_talk_id_before(
        &self,
        talk_id: &talk::Id,
        before: NaiveDateTime,
    ) -> super::Result<Vec<Message>> {
        let mut conn = self.pool.get()?;

        let msgs = messages::table
            .filter(
                messages::talk_id
                    .eq(talk_id.0)
                    .and(messages::created_at.lt(before)),
            )
            .order(messages::created_at.desc())
            .select(Message::as_select())
            .get_results(&mut conn)?;

        Ok(msgs)
    }

    fn find_by_talk_id_limited_before(
        &self,
        talk_id: &talk::Id,
        limit: i64,
        before: NaiveDateTime,
    ) -> super::Result<Vec<Message>> {
        let mut conn = self.pool.get()?;

        let msgs = messages::table
            .filter(
                messages::talk_id
                    .eq(talk_id.0)
                    .and(messages::created_at.lt(before)),
            )
            .limit(limit)
            .order(messages::created_at.desc())
            .select(Message::as_select())
            .get_results(&mut conn)?;

        Ok(msgs)
    }

    fn find_most_recent(&self, talk_id: &talk::Id) -> super::Result<Option<Message>> {
        let mut conn = self.pool.get()?;

        let msg = messages::table
            .filter(messages::talk_id.eq(talk_id.0))
            .limit(1)
            .order(messages::created_at.desc())
            .select(Message::as_select())
            .first(&mut conn)
            .optional()?;

        Ok(msg)
    }

    fn update(&self, id: &Id, content: &str) -> super::Result<bool> {
        let mut conn = self.pool.get()?;

        let res = diesel::update(messages::table.find(id.0))
            .set(messages::content.eq(content))
            .execute(&mut conn)?;

        Ok(res > 0)
    }

    fn delete(&self, id: &Id) -> super::Result<bool> {
        let mut conn = self.pool.get()?;

        let deleted_count = diesel::delete(messages::table.find(id.0)).execute(&mut conn)?;

        Ok(deleted_count > 0)
    }

    fn delete_by_talk_id(&self, talk_id: &talk::Id) -> super::Result<usize> {
        let mut conn = self.pool.get()?;

        let deleted_count = diesel::delete(messages::table.filter(messages::talk_id.eq(talk_id.0)))
            .execute(&mut conn)?;

        Ok(deleted_count)
    }

    fn mark_as_seen(&self, ids: &[Id]) -> super::Result<usize> {
        let mut conn = self.pool.get()?;

        let ids = ids.iter().map(|id| id.0).collect::<Vec<Uuid>>();

        let modified_count = diesel::update(messages::table.filter(messages::id.eq_any(ids)))
            .set(messages::seen.eq(true))
            .execute(&mut conn)?;

        Ok(modified_count)
    }
}
