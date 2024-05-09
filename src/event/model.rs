use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Login { token: String },
    CreateMessage { recipient: String, text: String },
    EditMessage { id: String, text: String },
    DeleteMessage { id: String },
}
