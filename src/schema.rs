// @generated automatically by Diesel CLI.

diesel::table! {
    contacts (user_id_1, user_id_2) {
        user_id_1 -> Uuid,
        user_id_2 -> Uuid,
        status -> Text,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        sub -> Text,
        nickname -> Text,
        picture -> Text,
        email -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    contacts,
    users,
);
