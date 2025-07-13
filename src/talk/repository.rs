use diesel::{
    Connection, ExpressionMethods, JoinOnDsl, NullableExpressionMethods, OptionalExtension,
    PgConnection, QueryDsl, QueryResult, RunQueryDsl, dsl::delete, insert_into,
    r2d2::ConnectionManager, sql_query, sql_types,
};

use crate::{
    message::{self, model::Message},
    schema::{
        chats, chats_users, groups, groups_users,
        talks::dsl::{id, kind, last_message_id, talks},
    },
    talk::{
        self, Kind,
        model::{
            ChatTalk, Details, GroupTalk, NewChat, NewChatUser, NewGroup, NewGroupUser, NewTalk,
        },
    },
    user,
};

use super::model::ChatWithLastMessage;

pub trait TalkRepository {
    fn find_chats_by_user_id(&self, user_id: &user::Id) -> super::Result<Vec<ChatTalk>>;

    fn find_groups_by_user_id(&self, user_id: &user::Id) -> super::Result<Vec<GroupTalk>>;

    fn find_chat_by_id_and_user_id(
        &self,
        id: &talk::Id,
        user_id: &user::Id,
    ) -> super::Result<Option<ChatTalk>>;

    fn find_group_by_id_and_user_id(
        &self,
        id: &talk::Id,
        user_id: &user::Id,
    ) -> super::Result<Option<GroupTalk>>;

    fn create(&self, t: &NewTalk) -> super::Result<talk::Id>;

    fn delete(&self, owner: &user::Id, id: &talk::Id) -> super::Result<bool>;

    fn exists(&self, members: &[user::Id; 2]) -> super::Result<bool>;
}

#[derive(Clone)]
pub struct PgTalkRepository {
    pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}

impl PgTalkRepository {
    pub const fn new(pool: r2d2::Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }
}

impl TalkRepository for PgTalkRepository {
    fn find_chats_by_user_id(&self, u_id: &user::Id) -> super::Result<Vec<ChatTalk>> {
        let mut conn = self.pool.get()?;

        let res: Vec<ChatWithLastMessage> = sql_query(
            r"
            SELECT
               	t.id,
               	m.id AS message_id,
               	m.owner,
               	m.content,
               	m.seen,
               	m.created_at,
               	u.id AS recipient,
               	u.name,
               	u.picture
            FROM talks t
            JOIN chats_users cu_self ON cu_self.chat_id = t.id AND cu_self.user_id = $1
            JOIN chats_users cu_other ON cu_other.chat_id = t.id AND cu_other.user_id != $1
            JOIN users u ON u.id = cu_other.user_id
            LEFT JOIN messages m ON m.id = t.last_message_id
            WHERE t.kind = 'chat'
            ",
        )
        .bind::<sql_types::Uuid, _>(u_id.get())
        .load::<ChatWithLastMessage>(&mut conn)?;

        Ok(res.into_iter().map(ChatTalk::from).collect())
    }

    fn find_groups_by_user_id(&self, u_id: &user::Id) -> super::Result<Vec<GroupTalk>> {
        use crate::schema::groups::dsl as g;
        use crate::schema::groups_users::dsl as gu;
        use crate::schema::messages::dsl as m;

        let mut conn = self.pool.get()?;

        let res: Vec<(talk::Id, Option<Message>, user::Id, String)> = talks
            .filter(kind.eq(Kind::Group))
            .filter(gu::user_id.eq(u_id))
            .inner_join(g::groups.inner_join(gu::groups_users))
            .left_join(m::messages.on(last_message_id.eq(m::id.nullable())))
            .select((
                id,
                (
                    m::id,
                    m::talk_id,
                    m::owner,
                    m::content,
                    m::created_at,
                    m::seen,
                )
                    .nullable(),
                g::owner,
                g::name,
            ))
            .get_results(&mut conn)?;

        Ok(res
            .into_iter()
            .map(|r| GroupTalk::new(r.0, r.1, r.2, r.3))
            .collect())
    }

    fn find_chat_by_id_and_user_id(
        &self,
        t_id: &talk::Id,
        u_id: &user::Id,
    ) -> super::Result<Option<ChatTalk>> {
        let mut conn = self.pool.get()?;

        let query = sql_query(
            r"
            SELECT
                t.id,
                m.id AS message_id,
               	m.owner,
               	m.content,
               	m.seen,
               	m.created_at,
                u.id AS recipient,
                u.name,
                u.picture
            FROM talks t
            JOIN chats_users cu_self ON cu_self.chat_id = t.id AND cu_self.user_id = $1
            JOIN chats_users cu_other ON cu_other.chat_id = t.id AND cu_other.user_id != $1
            JOIN users u ON u.id = cu_other.user_id
            LEFT JOIN messages m ON m.id = t.last_message_id
            WHERE t.id = $2
            AND t.kind = 'chat'
            ",
        )
        .bind::<sql_types::Uuid, _>(u_id.get())
        .bind::<sql_types::Uuid, _>(t_id.get());

        query
            .get_result::<ChatWithLastMessage>(&mut conn)
            .map(ChatTalk::from)
            .optional()
            .map_err(super::Error::from)
    }

    fn find_group_by_id_and_user_id(
        &self,
        t_id: &talk::Id,
        u_id: &user::Id,
    ) -> super::Result<Option<GroupTalk>> {
        use crate::schema::groups::dsl as g;
        use crate::schema::groups_users::dsl as gu;
        use crate::schema::messages::dsl as m;

        let mut conn = self.pool.get()?;

        let res: Option<(talk::Id, Option<Message>, user::Id, String)> = talks
            .filter(id.eq(t_id))
            .filter(kind.eq(Kind::Group))
            .filter(gu::user_id.eq(u_id))
            .inner_join(g::groups.inner_join(gu::groups_users))
            .left_join(m::messages.on(last_message_id.eq(m::id.nullable())))
            .select((
                id,
                (
                    m::id,
                    m::talk_id,
                    m::owner,
                    m::content,
                    m::created_at,
                    m::seen,
                )
                    .nullable(),
                g::owner,
                g::name,
            ))
            .get_result(&mut conn)
            .optional()?;

        Ok(res.map(|r| GroupTalk::new(r.0, r.1, r.2, r.3)))
    }

    fn create(&self, t: &NewTalk) -> super::Result<talk::Id> {
        let mut conn = self.pool.get()?;

        let tx_res: QueryResult<talk::Id> = conn.transaction(|conn| {
            let k = match t.details() {
                Details::Chat { .. } => Kind::Chat,
                Details::Group { .. } => Kind::Group,
            };

            let new_talk = (kind.eq(k), last_message_id.eq::<Option<message::Id>>(None));
            let t_id: talk::Id = insert_into(talks)
                .values(new_talk)
                .returning(id)
                .get_result(conn)?;

            match t.details() {
                Details::Chat { members } => {
                    let c_id: talk::Id = insert_into(chats::table)
                        .values(NewChat::new(&t_id))
                        .returning(chats::id)
                        .get_result(conn)?;

                    let users: Vec<NewChatUser> =
                        members.iter().map(|m| NewChatUser::new(&c_id, m)).collect();

                    insert_into(chats_users::table)
                        .values(users)
                        .execute(conn)?;
                }
                Details::Group {
                    name,
                    owner,
                    members,
                } => {
                    let g_id: talk::Id = insert_into(groups::table)
                        .values(NewGroup::new(&t_id, owner, name))
                        .returning(groups::id)
                        .get_result(conn)?;

                    let users: Vec<NewGroupUser> = members
                        .iter()
                        .map(|m| NewGroupUser::new(&g_id, m))
                        .collect();

                    insert_into(groups_users::table)
                        .values(users)
                        .execute(conn)?;
                }
            }
            Ok(t_id)
        });

        tx_res.map_err(super::Error::from)
    }

    fn delete(&self, _o: &user::Id, t_id: &talk::Id) -> super::Result<bool> {
        let mut conn = self.pool.get()?;

        // TODO: check if
        // 1. chat -> user is a member
        // 2. group -> user is owner
        let deleted_count = delete(talks.find(t_id)).execute(&mut conn)?;

        Ok(deleted_count > 0)
    }

    fn exists(&self, members: &[user::Id; 2]) -> super::Result<bool> {
        use crate::schema::chats_users::dsl::{chat_id, chats_users, user_id};
        use diesel::dsl::count_distinct;

        let mut conn = self.pool.get()?;

        chats_users
            .filter(user_id.eq_any(members))
            .select(chat_id)
            .group_by(chat_id)
            .having(count_distinct(user_id).eq(2))
            .first::<talk::Id>(&mut conn)
            .optional()
            .map(|r| r.is_some())
            .map_err(super::Error::from)
    }
}
