// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "talk_kind"))]
    pub struct TalkKind;
}

diesel::table! {
    chats (id) {
        id -> Uuid,
    }
}

diesel::table! {
    chats_users (chat_id, user_id) {
        chat_id -> Uuid,
        user_id -> Uuid,
    }
}

diesel::table! {
    contacts (id) {
        id -> Uuid,
        user_id_1 -> Uuid,
        user_id_2 -> Uuid,
        status -> Text,
        initiator -> Nullable<Uuid>,
    }
}

diesel::table! {
    groups (id) {
        id -> Uuid,
        owner -> Uuid,
        name -> Text,
    }
}

diesel::table! {
    groups_users (group_id, user_id) {
        group_id -> Uuid,
        user_id -> Uuid,
    }
}

diesel::table! {
    messages (id) {
        id -> Uuid,
        talk_id -> Uuid,
        owner -> Uuid,
        content -> Text,
        created_at -> Timestamp,
        seen -> Bool,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::TalkKind;

    talks (id) {
        id -> Uuid,
        kind -> TalkKind,
        last_message_id -> Nullable<Uuid>,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        sub -> Text,
        nickname -> Text,
        name -> Text,
        picture -> Text,
        email -> Text,
    }
}

diesel::joinable!(chats -> talks (id));
diesel::joinable!(chats_users -> chats (chat_id));
diesel::joinable!(chats_users -> users (user_id));
diesel::joinable!(groups -> talks (id));
diesel::joinable!(groups -> users (owner));
diesel::joinable!(groups_users -> groups (group_id));
diesel::joinable!(groups_users -> users (user_id));
diesel::joinable!(messages -> users (owner));

diesel::allow_tables_to_appear_in_same_query!(
    chats,
    chats_users,
    contacts,
    groups,
    groups_users,
    messages,
    talks,
    users,
);
