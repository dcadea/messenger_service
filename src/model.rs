use serde::Serialize;

pub struct AppEndpoints {
    address: String,
    port: String,
    api_path: String,
}

impl AppEndpoints {
    pub fn new(address: &str, port: &str, api_path: &str) -> Self {
        Self {
            address: address.to_string(),
            port: port.to_string(),
            api_path: api_path.to_string(),
        }
    }

    pub fn api(&self) -> String {
        format!("http://{}:{}/{}", self.address, self.port, self.api_path)
    }
}

#[derive(Serialize)]
pub struct Link {
    rel: String,
    href: String,
}

impl Link {
    pub fn _self(path: &str) -> Self {
        Self::new("self", path)
    }

    pub fn recipient(path: &str) -> Self {
        Self::new("recipient", path)
    }
}

impl Link {
    fn new(rel: &str, path: &str) -> Self {
        Self {
            rel: rel.to_string(),
            // TODO: get the base url from configuration
            href: format!("http://127.0.0.1:8000/api/v1{}", path),
        }
    }
}
