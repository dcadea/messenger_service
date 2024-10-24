use mongodb::bson::serde_helpers::hex_string_as_object_id;
use serde::{Deserialize, Serialize};

mod model;

#[derive(Deserialize, Serialize)]
struct Id(#[serde(with = "hex_string_as_object_id")] String);
