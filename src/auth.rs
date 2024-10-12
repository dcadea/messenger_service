use self::service::AuthService;
use crate::state::AppState;
use crate::user::model::UserInfo;
use crate::user::service::UserService;
use crate::{integration, user};
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use serde::Deserialize;

use axum::http::StatusCode;

type Result<T> = std::result::Result<T, Error>;

#[derive(Deserialize, Clone)]
struct TokenClaims {
    sub: user::Sub,
}

pub(crate) fn endpoints<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/login", get(handler::login))
        .route(
            "/logout",
            get(|| async { (StatusCode::OK, "Mocking logout :)") }),
        )
        .route("/callback", get(handler::callback))
        .with_state(state)
}

pub(self) mod handler {
    use super::service::AuthService;
    use axum::http::StatusCode;
    use axum::{
        extract::State,
        response::{IntoResponse, Redirect},
    };
    use axum_extra::extract::Query;
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct Params {
        code: String,
        state: String,
    }

    pub async fn login(auth_service: State<AuthService>) -> crate::Result<impl IntoResponse> {
        Ok(Redirect::to(&auth_service.authorize().await))
    }

    pub async fn callback(
        params: Query<Params>,
        auth_service: State<AuthService>,
    ) -> crate::Result<impl IntoResponse> {
        let token = auth_service.exchange_code(&params.code).await?;
        Ok((StatusCode::OK, token)) // TODO: set in session storage
    }
}

pub(crate) mod service {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    use jsonwebtoken::jwk::JwkSet;
    use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};
    use oauth2::basic::BasicClient;
    use oauth2::reqwest::async_http_client;
    use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
    use tokio::sync::RwLock;

    use super::TokenClaims;

    use crate::integration::idp;
    use crate::user::model::UserInfo;
    use crate::{auth, integration};

    const ONE_DAY: Duration = Duration::from_secs(24 * 60 * 60);

    #[derive(Clone)]
    pub struct AuthService {
        config: Arc<idp::Config>,
        http: Arc<reqwest::Client>,
        oauth2: Arc<BasicClient>,
        jwt_validator: Arc<Validation>,
        jwk_decoding_keys: Arc<RwLock<HashMap<String, DecodingKey>>>,
    }

    impl AuthService {
        pub fn try_new(config: &idp::Config) -> super::Result<Self> {
            let mut jwt_validator = Validation::new(jsonwebtoken::Algorithm::RS256);
            jwt_validator.set_required_spec_claims(&config.required_claims);
            jwt_validator.set_issuer(&[&config.issuer]);
            jwt_validator.set_audience(&[&config.audience]);

            let jwk_decoding_keys = Arc::new(RwLock::new(HashMap::new()));
            let service = Self {
                config: Arc::new(config.to_owned()),
                http: Arc::new(integration::init_http_client()?),
                oauth2: Arc::new(integration::idp::init(config)),
                jwt_validator: Arc::new(jwt_validator),
                jwk_decoding_keys: jwk_decoding_keys.clone(),
            };

            // TODO: uncomment
            // let http = integration::init_http_client()?;
            // let config_clone = config.clone();
            // tokio::spawn(async move {
            //     loop {
            //         match fetch_jwk_decoding_keys(&config_clone, &http).await {
            //             Ok(keys) => *jwk_decoding_keys.write().await = keys,
            //             Err(e) => eprintln!("Failed to update JWK decoding keys: {:?}", e),
            //         }
            //         sleep(ONE_DAY).await;
            //     }
            // });

            Ok(service)
        }
    }

    impl AuthService {
        pub async fn authorize(&self) -> String {
            let (auth_url, _) = self // TODO: use csrf_token
                .oauth2
                .authorize_url(CsrfToken::new_random)
                .add_extra_param("audience", self.config.audience.clone())
                .add_scopes([
                    Scope::new("openid".to_string()),
                    Scope::new("profile".to_string()),
                ])
                .url();
            auth_url.to_string()
        }

        pub async fn exchange_code(&self, code: &str) -> super::Result<String> {
            let code = AuthorizationCode::new(code.to_string());

            let token_result = self
                .oauth2
                .exchange_code(code)
                .request_async(async_http_client)
                .await
                .expect("Failed to exchange code"); // FIXME: handle error

            Ok(token_result.access_token().secret().to_owned())
        }

        pub async fn validate(&self, token: &str) -> super::Result<TokenClaims> {
            let kid = self.get_kid(token)?;
            let decoding_keys_guard = self.jwk_decoding_keys.read().await;
            let decoding_key = decoding_keys_guard
                .get(&kid)
                .ok_or(auth::Error::UnknownKid)?;

            decode::<TokenClaims>(token, decoding_key, &self.jwt_validator)
                .map(|data| data.claims)
                .map_err(|e| auth::Error::Forbidden(e.to_string()))
        }

        pub async fn get_user_info(&self, token: &str) -> super::Result<UserInfo> {
            let user_info = self
                .http
                .get(&self.config.userinfo_url)
                .bearer_auth(token)
                .send()
                .await?
                .json::<UserInfo>()
                .await?;

            Ok(user_info)
        }
    }

    impl AuthService {
        fn get_kid(&self, token: &str) -> super::Result<String> {
            let jwt_header =
                decode_header(token).map_err(|e| auth::Error::TokenMalformed(e.to_string()))?;

            jwt_header
                .kid
                .map(|kid| kid.to_string())
                .ok_or(auth::Error::UnknownKid)
        }
    }

    async fn fetch_jwk_decoding_keys(
        config: &idp::Config,
        http: &reqwest::Client,
    ) -> super::Result<HashMap<String, DecodingKey>> {
        let jwk_response = http.get(&config.jwks_url).send().await?;
        let jwk_json = jwk_response.json().await?;
        let jwk_set: JwkSet = serde_json::from_value(jwk_json)?;

        let mut jwk_decoding_keys = HashMap::new();

        for jwk in jwk_set.keys.iter() {
            if let Some(kid) = jwk.clone().common.key_id {
                let key = DecodingKey::from_jwk(jwk)
                    .map_err(|e| auth::Error::Unexpected(e.to_string()))?;
                jwk_decoding_keys.insert(kid, key);
            }
        }

        Ok(jwk_decoding_keys)
    }
}

pub(crate) async fn validate_token(
    auth_service: State<AuthService>,
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
    mut request: Request,
    next: Next,
) -> crate::Result<Response> {
    let auth_header = auth_header.ok_or(Error::Unauthorized)?;
    let claims = auth_service.validate(auth_header.token()).await?;
    request.extensions_mut().insert(claims);

    let response = next.run(request).await;
    Ok(response)
}

// TODO: remove this function
pub(crate) async fn set_test_user_context(
    user_service: State<UserService>,
    mut request: Request,
    next: Next,
) -> crate::Result<Response> {
    let user_info = user_service
        .find_user_info(&user::Sub("github|10639696".to_string()))
        .await?;

    request.extensions_mut().insert(user_info);

    let response = next.run(request).await;
    Ok(response)
}

pub(crate) async fn set_user_context(
    user_service: State<UserService>,
    auth_service: State<AuthService>,
    auth_header: TypedHeader<Authorization<Bearer>>,
    mut request: Request,
    next: Next,
) -> crate::Result<Response> {
    let claims = request
        .extensions()
        .get::<TokenClaims>()
        .ok_or(Error::Unauthorized)?;

    let user_info = match user_service.find_user_info(&claims.sub).await {
        Ok(user_info) => user_info,
        Err(user::Error::NotFound(_)) => {
            let user_info = auth_service.get_user_info(auth_header.token()).await?;
            let user = user_info.clone().into();
            user_service.create(&user).await?;
            user_info
        }
        Err(e) => return Err(e.into()),
    };

    request.extensions_mut().insert(user_info);

    let response = next.run(request).await;
    Ok(response)
}

pub(crate) async fn cache_user_friends(
    user_service: State<UserService>,
    request: Request,
    next: Next,
) -> crate::Result<Response> {
    let user_info = request
        .extensions()
        .get::<UserInfo>()
        .ok_or(Error::Unauthorized)?;

    user_service.cache_friends(&user_info.sub).await?;

    let response = next.run(request).await;
    Ok(response)
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub(crate) enum Error {
    #[error("unauthorized to access the resource")]
    Unauthorized,
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("missing or unknown kid")]
    UnknownKid,
    #[error("token is malformed: {0}")]
    TokenMalformed(String),
    #[error("unexpected auth error: {0}")]
    Unexpected(String),

    _User(#[from] user::Error),
    _Integration(#[from] integration::Error),

    _Reqwest(#[from] reqwest::Error),
    _ParseJson(#[from] serde_json::Error),
}
