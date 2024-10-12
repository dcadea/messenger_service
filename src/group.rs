mod model {
    use serde::{Deserialize, Serialize};

    use crate::user::model::Sub;
    use crate::util::serialize_object_id;

    type GroupId = mongodb::bson::oid::ObjectId;

    #[derive(Serialize, Deserialize)]
    struct Group {
        #[serde(
            alias = "_id",
            serialize_with = "serialize_object_id",
            skip_serializing_if = "Option::is_none"
        )]
        id: Option<GroupId>,
        name: String,
        owner: Sub,
        participants: Vec<Sub>,
        picture: String,
        last_message: String,
    }
}
