use std::env;

use log::warn;
use minio::s3::{
    args::{BucketExistsArgs, MakeBucketArgs},
    client::ClientBuilder,
    creds::StaticProvider,
    http::BaseUrl,
};

#[derive(Clone)]
pub struct S3 {
    client: minio::s3::client::Client,
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
            Err(e) => panic!("Failed to connect to MINIO: {e}"),
        };

        let provider = StaticProvider::from(self.credentials.clone());

        let client = match ClientBuilder::new(base_url)
            .provider(Some(Box::new(provider)))
            .build()
        {
            Ok(c) => c,
            Err(e) => panic!("Failed to connect to MINIO: {e}"),
        };

        // TODO: handle errors
        let bucket = "messenger";
        let exists = client
            .bucket_exists(&BucketExistsArgs::new(bucket).unwrap())
            .await
            .unwrap();

        if !exists {
            client
                .make_bucket(&MakeBucketArgs::new(bucket).unwrap())
                .await
                .unwrap();
        }

        S3 { client }
    }
}
