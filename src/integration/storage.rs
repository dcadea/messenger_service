use std::env;

use log::warn;
use minio::s3::{
    builders::ObjectContent, client::ClientBuilder, creds::StaticProvider, http::BaseUrl,
    types::S3Api,
};

const BUCKET: &str = "messenger";

#[derive(Clone)]
pub struct S3 {
    client: minio::s3::client::Client,
}

impl S3 {
    pub async fn generate(&self, blob: Blob<'_>) -> super::Result<()> {
        match blob {
            Blob::Png(name) => {
                let png = identicon_rs::Identicon::new(name).export_png_data()?;
                let png = ObjectContent::from(png);

                self.client
                    .put_object_content(BUCKET, blob, png)
                    .send()
                    .await
                    .map_err(Box::new)?;
            }
        }
        Ok(())
    }

    pub async fn find_one(&self, blob: Blob<'_>) -> super::Result<ObjectContent> {
        let res = self
            .client
            .get_object(BUCKET, blob)
            .send()
            .await
            .map_err(Box::new)?;
        Ok(res.content)
    }
}

pub enum Blob<'a> {
    Png(&'a str),
}

impl<'a> From<Blob<'a>> for String {
    fn from(b: Blob<'a>) -> Self {
        match b {
            Blob::Png(name) => format!("{name}.png"),
        }
    }
}

#[derive(Clone)]
struct Credentials {
    user: String,
    password: String,
}

impl Default for Credentials {
    fn default() -> Self {
        Self {
            user: String::from("minioadmin"),
            password: String::from("minioadmin"),
        }
    }
}

impl From<Credentials> for StaticProvider {
    fn from(c: Credentials) -> Self {
        Self::new(&c.user, &c.password, None)
    }
}

#[derive(Clone)]
pub struct Config {
    host: String,
    port: u16,
    credentials: Credentials,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: String::from("127.0.0.1"),
            port: 9000,
            credentials: Credentials::default(),
        }
    }
}

impl Config {
    pub fn env() -> Option<Self> {
        let host = env::var("MINIO_HOST").ok();
        let port = env::var("MINIO_PORT")
            .unwrap_or_else(|_| "9000".to_string())
            .parse()
            .ok();

        if let (Some(host), Some(port)) = (host, port) {
            let credentials = env::var("MINIO_USER")
                .and_then(|user| {
                    env::var("MINIO_PASSWORD").map(|password| Credentials { user, password })
                })
                .unwrap_or_default();

            Some(Self {
                host,
                port,
                credentials,
            })
        } else {
            warn!("MINIO env is not configured");
            None
        }
    }

    pub async fn connect(&self) -> S3 {
        let base_url = match format!("http://{}:{}/", self.host, self.port).parse::<BaseUrl>() {
            Ok(url) => url,
            Err(e) => panic!("Failed to connect to MINIO: {e:?}"),
        };

        let provider = StaticProvider::from(self.credentials.clone());

        let client = match ClientBuilder::new(base_url)
            .provider(Some(Box::new(provider)))
            .build()
        {
            Ok(c) => c,
            Err(e) => panic!("Failed to connect to MINIO: {e:?}"),
        };

        let exists = client
            .bucket_exists(BUCKET)
            .send()
            .await
            .map(|r| r.exists)
            .unwrap_or(false);

        if !exists {
            if let Err(e) = client.create_bucket(BUCKET).send().await {
                panic!("Failed to create MINIO bucket: {BUCKET}, {e:?}")
            }
        }

        S3 { client }
    }
}
