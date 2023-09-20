use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Validate, ToSchema)]
pub struct AuthToken {
    pub expires_in: i32,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Validate, ToSchema)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Validate, ToSchema)]
pub struct EmailUser {
    pub id: i64,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    #[serde(skip_serializing)]
    pub salt: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
