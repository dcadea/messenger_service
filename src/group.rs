mod model {
    use serde::{Deserialize, Serialize};

    use crate::user;
    use messenger_service::serde::serialize_object_id;

    type Id = mongodb::bson::oid::ObjectId;

    #[derive(Serialize, Deserialize)]
    struct Group {
        #[serde(
            alias = "_id",
            serialize_with = "serialize_object_id",
            skip_serializing_if = "Option::is_none"
        )]
        id: Option<Id>,
        name: String,
        owner: user::Sub,
        participants: Vec<user::Sub>,
        picture: String,
        last_message: String,
    }
}
