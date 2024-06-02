use crate::user::model::UserSub;
use crate::util::serialize_object_id;
use serde::{Deserialize, Serialize};

type GroupId = mongodb::bson::oid::ObjectId;
type Participants = Vec<UserSub>;

#[derive(Serialize, Deserialize)]
pub struct Group {
    #[serde(
        alias = "_id",
        serialize_with = "serialize_object_id",
        skip_serializing_if = "Option::is_none"
    )]
    id: Option<GroupId>,
    name: String,
    owner: UserSub,
    participants: Participants,
    picture: String,
    last_message: String,
}
