use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use super::error::AuthError;

#[derive(Debug, Deserialize, Serialize)]
pub struct Claims {
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
}

impl Claims {
    pub fn new(sub: &str, iat: DateTime<Utc>, exp: DateTime<Utc>) -> Self {
        Self {
            sub: sub.to_string(),
            iat: iat.timestamp(),
            exp: exp.timestamp(),
        }
    }
}

pub fn sign(secret: &str, sub: &str, expires_in: Duration, iat: DateTime<Utc>) -> Result<String, AuthError> {
    let exp = iat + expires_in;
    Ok(jsonwebtoken::encode(
        &Header::default(),
        &Claims::new(sub, iat, exp),
        &EncodingKey::from_secret(secret.as_bytes()),
    )?)
}

pub fn verify(secret: &str, token: &str, now: DateTime<Utc>) -> Result<Claims, AuthError> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    let validation = Validation::default();
    let claims: Claims = jsonwebtoken::decode(token, &key, &validation).map(|token| token.claims)?;

    let expired_at = NaiveDateTime::from_timestamp_opt(claims.exp, 0).unwrap_or(NaiveDateTime::MIN);
    let expired_at: DateTime<Utc> = Utc.from_utc_datetime(&expired_at);
    if expired_at < now {
        return Err(AuthError::AccessTokenExpired);
    }

    Ok(claims)
}
