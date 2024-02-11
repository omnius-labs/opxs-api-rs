use std::sync::Arc;

use chrono::Utc;
use sqlx::PgPool;

use core_base::{clock::SystemClock, tsid::TsidProvider};

use opxs_base::AppError;

use crate::shared::model::{EmailUser, UserAuthenticationType, UserRole};

pub struct EmailAuthRepo {
    pub db: Arc<PgPool>,
    pub system_clock: Arc<dyn SystemClock<Utc> + Send + Sync>,
    pub tsid_provider: Arc<dyn TsidProvider + Send + Sync>,
}

impl EmailAuthRepo {
    pub async fn create_user(&self, name: &str, email: &str, password_hash: &str, salt: &str) -> Result<String, AppError> {
        let user_id = self.tsid_provider.gen().to_string();
        let now = self.system_clock.now();

        let mut tx = self.db.begin().await?;

        sqlx::query(
            r#"
INSERT INTO users (id, name, authentication_type, role, created_at, updated_at)
    VALUES ($1, $2, $3, $4, $5, $6)
"#,
        )
        .bind(&user_id)
        .bind(name)
        .bind(UserAuthenticationType::Email)
        .bind(UserRole::User)
        .bind(now)
        .bind(now)
        .execute(&mut tx)
        .await
        .map_err(|e| AppError::UnexpectedError(e.into()))?;

        sqlx::query(
            r#"
INSERT INTO user_auth_emails (user_id, email, password_hash, salt, created_at, updated_at)
    VALUES ($1, $2, $3, $4, $5, $6)
    ON CONFLICT (email)
    DO UPDATE SET
        user_id = $1,
        password_hash = $3,
        salt = $4,
        updated_at = $6;
"#,
        )
        .bind(&user_id)
        .bind(email)
        .bind(password_hash)
        .bind(salt)
        .bind(now)
        .bind(now)
        .execute(&mut tx)
        .await
        .map_err(|e| AppError::UnexpectedError(e.into()))?;

        tx.commit().await?;

        Ok(user_id)
    }

    pub async fn delete_user(&self, id: &str) -> Result<(), AppError> {
        let mut tx = self.db.begin().await?;

        let queries = vec![
            sqlx::query("DELETE FROM users WHERE id = $1").bind(id),
            sqlx::query("DELETE FROM user_auth_emails WHERE user_id = $1").bind(id),
            sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1").bind(id),
        ];

        for query in queries {
            query.execute(&mut tx).await.map_err(|e| AppError::UnexpectedError(e.into()))?;
        }

        tx.commit().await?;

        Ok(())
    }

    pub async fn exist_user(&self, email: &str) -> Result<bool, AppError> {
        let (existed,): (bool,) = sqlx::query_as(
            r#"
SELECT EXISTS (
    SELECT u.id
        FROM users u
        JOIN user_auth_emails e on u.id = e.user_id
        WHERE e.email = $1 AND e.email_verified = true
        LIMIT 1
);
"#,
        )
        .bind(email)
        .fetch_one(self.db.as_ref())
        .await
        .map_err(|e| AppError::UnexpectedError(e.into()))?;

        Ok(existed)
    }

    pub async fn get_user(&self, email: &str) -> Result<EmailUser, AppError> {
        let user: Option<EmailUser> = sqlx::query_as(
            r#"
SELECT u.id, u.name, u.role, e.email, e.password_hash, e.salt, u.created_at, u.updated_at
    FROM users u
    JOIN user_auth_emails e on u.id = e.user_id
    WHERE e.email = $1 AND e.email_verified = true
    LIMIT 1;
"#,
        )
        .bind(email)
        .fetch_optional(self.db.as_ref())
        .await
        .map_err(|e| AppError::UnexpectedError(e.into()))?;

        if user.is_none() {
            return Err(AppError::UserNotFound);
        }

        Ok(user.unwrap())
    }

    pub async fn update_email_verified(&self, email: &str, email_verified: bool) -> Result<(), AppError> {
        let now = self.system_clock.now();

        sqlx::query(
            r#"
UPDATE user_auth_emails
    SET email_verified = $2, updated_at = $3
    WHERE email = $1;
"#,
        )
        .bind(email)
        .bind(email_verified)
        .bind(now)
        .execute(self.db.as_ref())
        .await
        .map_err(|e| AppError::UnexpectedError(e.into()))?;

        Ok(())
    }
}
