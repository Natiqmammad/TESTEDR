use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::{request::Parts, HeaderMap};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

use crate::{api::AppState, error::AppError};

#[derive(Clone)]
pub struct JwtKeys {
    encoding: EncodingKey,
    secret: Arc<Vec<u8>>,
}

impl JwtKeys {
    pub fn new(secret: impl Into<String>) -> Self {
        let secret = secret.into();
        let bytes = secret.into_bytes();
        Self {
            encoding: EncodingKey::from_secret(&bytes),
            secret: Arc::new(bytes),
        }
    }

    pub fn token(&self, user_id: i64, username: &str) -> Result<String, AppError> {
        let exp = SystemTime::now()
            .checked_add(Duration::from_secs(60 * 60 * 24))
            .ok_or_else(|| AppError::unauthorized("invalid exp"))?
            .duration_since(UNIX_EPOCH)
            .map_err(|_| AppError::unauthorized("invalid exp"))?
            .as_secs() as usize;
        let claims = Claims {
            sub: user_id,
            username: username.to_string(),
            exp,
        };
        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(|e| AppError::unauthorized(e.to_string()))
    }

    pub fn verify(&self, token: &str) -> Result<Claims, AppError> {
        let decoding = DecodingKey::from_secret(&self.secret);
        let data = decode::<Claims>(token, &decoding, &Validation::new(Algorithm::HS256))
            .map_err(|e| AppError::unauthorized(e.to_string()))?;
        Ok(data.claims)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64,
    pub username: String,
    pub exp: usize,
}

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .to_string();
    Ok(hash)
}

pub fn verify_password(hash: &str, password: &str) -> Result<(), AppError> {
    let parsed = PasswordHash::new(hash).map_err(|e| AppError::unauthorized(e.to_string()))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| AppError::unauthorized("invalid credentials"))
}

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub id: i64,
    pub username: String,
}

pub struct AuthExtractor(pub AuthUser);

#[async_trait]
impl FromRequestParts<AppState> for AuthExtractor {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let headers = parts.headers.clone();
        let token = extract_bearer(&headers)?;
        let claims = state.jwt.verify(&token)?;
        Ok(AuthExtractor(AuthUser {
            id: claims.sub,
            username: claims.username,
        }))
    }
}

fn extract_bearer(headers: &HeaderMap) -> Result<String, AppError> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)
        .ok_or_else(|| AppError::unauthorized("missing authorization header"))?
        .to_str()
        .map_err(|_| AppError::unauthorized("invalid authorization header"))?;
    if let Some(rest) = value.strip_prefix("Bearer ") {
        Ok(rest.to_string())
    } else {
        Err(AppError::unauthorized("invalid authorization header"))
    }
}
