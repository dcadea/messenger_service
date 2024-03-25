use mongodb::bson;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct User {
    #[serde(skip)]
    _id: Option<bson::oid::ObjectId>,
    username: String,
    password: String,
}

impl User {
    pub fn new(username: &str, password: &str) -> Self {
        Self {
            _id: None,
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    pub fn username(&self) -> &str {
        self.username.as_str()
    }

    pub fn password(&self) -> &str {
        self.password.as_str()
    }
}

#[derive(serde::Serialize)]
pub struct UserResponse {
    username: String,
}

impl UserResponse {
    pub fn new(username: &str) -> Self {
        Self { username: username.to_string() }
    }
}