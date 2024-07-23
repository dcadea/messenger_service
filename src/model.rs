use serde::Serialize;

#[derive(Clone)]
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

#[derive(Clone)]
pub struct LinkFactory {
    base_url: String,
}

impl LinkFactory {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    pub fn _self(&self, path: &str) -> Link {
        let href = format!("{}/{}", self.base_url, path);
        Link::new("self", &href)
    }

    pub fn recipient(&self, path: &str) -> Link {
        let href = format!("{}/{}", self.base_url, path);
        Link::new("recipient", &href)
    }
}

#[derive(Serialize)]
pub struct Link {
    rel: String,
    href: String,
}

impl Link {
    fn new(rel: &str, href: &str) -> Self {
        Self {
            rel: rel.to_string(),
            href: href.to_string(),
        }
    }
}
