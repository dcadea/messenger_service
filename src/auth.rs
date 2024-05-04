use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use jsonwebtoken::{decode_header, Header};
use jsonwebtoken::jwk::JwkSet;
use serde_json::Value;

use crate::state::AppState;

pub async fn validate_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    match get_token(&headers) {
        Some((token, header)) if token_is_valid((token, header.clone()), state).await => {
            let response = next.run(request).await;
            Ok(response)
        }
        _ => {
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

fn get_token(headers: &HeaderMap) -> Option<(&str, Header)> {
    match headers.get("Authorization") {
        None => return None,
        Some(authorization) => match authorization.to_str() {
            Ok(token) => {
                if let Some(token) = token.split_whitespace().last() {
                    if let Ok(decoded_header) = decode_header(token) {
                        return Some((token, decoded_header));
                    }
                }
                None
            }
            Err(_) => None,
        },
    }
}

// TODO: refactor
async fn token_is_valid(token_header: (&str, Header), state: AppState) -> bool {
    let http = state.http.clone();
    let jwk_response = http.get("https://dcadea.auth0.com/.well-known/jwks.json").send().await.unwrap();
    let jwk_json = jwk_response.json().await.unwrap();
    let jwk_set: JwkSet = serde_json::from_value(jwk_json).unwrap();

    let kid = token_header.1.kid.as_ref().unwrap();
    let jwk = jwk_set.find(&kid).unwrap();

    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_audience(&["https://messenger.angelwing.io/api/v1", "https://dcadea.auth0.com/api/v2/"]);
    let decoded = jsonwebtoken::decode::<Value>(
        token_header.0,
        &jsonwebtoken::DecodingKey::from_jwk(jwk).unwrap(),
        &validation,
    );

    decoded.is_ok()
}