pub mod serde {
    use mongodb::bson::oid::ObjectId;
    use serde::Serializer;

    pub fn serialize_object_id<S>(
        message_id: &Option<ObjectId>,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match message_id {
            Some(ref message_id) => serializer.serialize_some(message_id.to_hex().as_str()),
            None => serializer.serialize_none(),
        }
    }
}
