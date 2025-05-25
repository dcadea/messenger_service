use std::env;

use log::warn;
use minio::s3::{
    args::{BucketExistsArgs, MakeBucketArgs},
    client::ClientBuilder,
    creds::StaticProvider,
    http::BaseUrl,
};

const BUCKET: &str = "messenger";

#[derive(Clone)]
pub struct S3 {
    client: minio::s3::client::Client,
}

impl S3 {
    pub async fn save_icon(&self, name: &str, icon: identicon_rs::Identicon) -> String {
        unimplemented!()
        // TODO
        // let image = icon.generate_image().unwrap();
        // let mut bytes = image.as_bytes();
        // let size = Some(bytes.len());
        // let mut p = PutObjectArgs::new(BUCKET, name, &mut bytes, size, None).unwrap();
        // let res = self.client.put_object(&mut p).await.unwrap();
        // res.location
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
            .bucket_exists(&BucketExistsArgs::new(BUCKET).unwrap())
            .await
            .unwrap_or(false);

        if !exists {
            if let Err(e) = client
                .make_bucket(&MakeBucketArgs::new(BUCKET).unwrap())
                .await
            {
                panic!("Failed to create MINIO bucket: {BUCKET}, {e:?}")
            }
        }

        S3 { client }
    }
}
