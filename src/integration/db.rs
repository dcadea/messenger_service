pub mod pg {
    use std::env;

    use diesel::backend::Backend;
    use diesel::deserialize::FromSql;
    use diesel::serialize::{IsNull, Output, ToSql};
    use diesel::{deserialize, serialize, sql_types};
    use std::io::Write;
    use uuid::Uuid;

    use crate::schema::sql_types::TalkKind;
    use crate::{contact, message, talk, user};

    use diesel::{PgConnection, r2d2::ConnectionManager};
    use log::warn;

    #[derive(Clone)]
    pub struct Config {
        host: String,
        port: u16,
        db: String,
        credentials: Credentials,
    }

    impl Default for Config {
        fn default() -> Self {
            Self {
                host: String::from("127.0.0.1"),
                port: 5432,
                db: String::from("messenger"),
                credentials: Credentials::default(),
            }
        }
    }

    impl Config {
        pub fn env() -> Option<Self> {
            let host = env::var("POSTGRES_HOST").ok();
            let port = env::var("POSTGRES_HOST")
                .unwrap_or_else(|_| "5432".to_string())
                .parse()
                .ok();
            let db = env::var("POSTGRES_DB").unwrap_or_else(|_e| String::from("messenger"));
            let credentials = env::var("POSTGRES_USER")
                .and_then(|u| env::var("POSTGRES_PASSWORD").map(|p| (u, p)))
                .map(|(user, password)| Credentials { user, password })
                .unwrap_or_default();

            if let (Some(host), Some(port)) = (host, port) {
                Some(Self {
                    host,
                    port,
                    db,
                    credentials,
                })
            } else {
                warn!("POSTGRES env is not configured");
                None
            }
        }

        pub fn connect(&self) -> r2d2::Pool<ConnectionManager<PgConnection>> {
            let database_url = format!(
                "postgres://{}:{}@{}:{}/{}",
                self.credentials.user, self.credentials.password, self.host, self.port, self.db
            );

            let manager = ConnectionManager::<PgConnection>::new(database_url);

            match r2d2::Pool::builder().build(manager) {
                Ok(pool) => pool,
                Err(e) => panic!("Failed to connect to PostgreSQL: {e}"),
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
                user: String::from("postgres"),
                password: String::from("postgres"),
            }
        }
    }

    impl<DB> FromSql<TalkKind, DB> for talk::Kind
    where
        DB: Backend,
        String: FromSql<sql_types::Text, DB>,
    {
        fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
            let s = String::from_sql(bytes)?;
            match s.as_str() {
                "chat" => Ok(Self::Chat),
                "group" => Ok(Self::Group),
                other => Err(Box::new(talk::Error::UnsupportedKind(other.to_string()))),
            }
        }
    }

    impl ToSql<TalkKind, diesel::pg::Pg> for talk::Kind {
        fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> serialize::Result {
            match *self {
                Self::Chat => out.write_all(b"chat")?,
                Self::Group => out.write_all(b"group")?,
            }
            Ok(IsNull::No)
        }
    }

    impl<DB> FromSql<sql_types::Uuid, DB> for user::Id
    where
        DB: Backend,
        Uuid: FromSql<sql_types::Uuid, DB>,
    {
        fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
            Uuid::from_sql(bytes).map(Self::from)
        }
    }

    impl ToSql<sql_types::Uuid, diesel::pg::Pg> for user::Id {
        fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> serialize::Result {
            out.write_all(self.get().as_bytes())?;
            Ok(IsNull::No)
        }
    }

    impl<DB> FromSql<sql_types::Uuid, DB> for contact::Id
    where
        DB: Backend,
        Uuid: FromSql<sql_types::Uuid, DB>,
    {
        fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
            Uuid::from_sql(bytes).map(Self::from)
        }
    }

    impl ToSql<sql_types::Uuid, diesel::pg::Pg> for contact::Id {
        fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> serialize::Result {
            out.write_all(self.get().as_bytes())?;
            Ok(IsNull::No)
        }
    }

    impl<DB> FromSql<sql_types::Uuid, DB> for message::Id
    where
        DB: Backend,
        Uuid: FromSql<sql_types::Uuid, DB>,
    {
        fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
            Uuid::from_sql(bytes).map(Self::from)
        }
    }

    impl ToSql<sql_types::Uuid, diesel::pg::Pg> for message::Id {
        fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> serialize::Result {
            out.write_all(self.get().as_bytes())?;
            Ok(IsNull::No)
        }
    }

    impl<DB> FromSql<sql_types::Uuid, DB> for talk::Id
    where
        DB: Backend,
        Uuid: FromSql<sql_types::Uuid, DB>,
    {
        fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
            Uuid::from_sql(bytes).map(Self::from)
        }
    }

    impl ToSql<sql_types::Uuid, diesel::pg::Pg> for talk::Id {
        fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> serialize::Result {
            out.write_all(self.get().as_bytes())?;
            Ok(IsNull::No)
        }
    }
}
