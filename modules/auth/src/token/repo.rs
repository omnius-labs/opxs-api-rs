use std::sync::Arc;

use chrono::{DateTime, Utc};
use core_base::clock::SystemClock;
use sqlx::PgPool;

use crate::shared::{error::AuthError, model::User};

pub struct TokenRepo {
    pub db: Arc<PgPool>,
    pub system_clock: Arc<dyn SystemClock<Utc> + Send + Sync>,
}

impl TokenRepo {
    pub async fn create_token(&self, user_id: &str, refresh_token: &str, expires_at: &DateTime<Utc>) -> Result<(), AuthError> {
        let now = self.system_clock.now();
        sqlx::query(
            r#"
INSERT INTO refresh_tokens (refresh_token, user_id, expires_at, created_at, updated_at)
    VALUES ($1, $2, $3, $4, $5);
"#,
        )
        .bind(refresh_token)
        .bind(user_id)
        .bind(expires_at)
        .bind(now)
        .bind(now)
        .execute(self.db.as_ref())
        .await
        .map_err(|e| AuthError::UnexpectedError(e.into()))?;

        Ok(())
    }

    pub async fn delete_token(&self, refresh_token: &str) -> Result<(), AuthError> {
        sqlx::query(
            r#"
DELETE FROM refresh_tokens
    WHERE refresh_token = $1;
"#,
        )
        .bind(refresh_token)
        .execute(self.db.as_ref())
        .await
        .map_err(|e| AuthError::UnexpectedError(e.into()))?;

        Ok(())
    }

    pub async fn update_token(&self, refresh_token: &str, expires_at: &DateTime<Utc>) -> Result<(), AuthError> {
        let now = self.system_clock.now();
        sqlx::query(
            r#"
UPDATE refresh_tokens
    SET expires_at = $2, updated_at = $3
    WHERE refresh_token = $1;
"#,
        )
        .bind(refresh_token)
        .bind(expires_at)
        .bind(now)
        .execute(self.db.as_ref())
        .await
        .map_err(|e| AuthError::UnexpectedError(e.into()))?;

        Ok(())
    }

    pub async fn get_user_id(&self, refresh_token: &str) -> Result<String, AuthError> {
        let now = self.system_clock.now();
        let user: Option<User> = sqlx::query_as(
            r#"
SELECT u.*
    FROM users u
    JOIN refresh_tokens t on t.user_id = u.id
    WHERE t.refresh_token = $1 AND expires_at > $2;
"#,
        )
        .bind(refresh_token)
        .bind(now)
        .fetch_optional(self.db.as_ref())
        .await
        .map_err(|e| AuthError::UnexpectedError(e.into()))?;

        if user.is_none() {
            return Err(AuthError::RefreshTokenNotFound);
        }

        Ok(user.unwrap().id)
    }
}
