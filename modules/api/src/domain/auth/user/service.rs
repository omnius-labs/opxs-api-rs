use std::sync::Arc;

use crate::{common::AppError, domain::auth::model::User};

use super::UserRepo;

pub struct UserService {
    pub user_repo: Arc<UserRepo>,
}

impl UserService {
    pub async fn get_user(&self, user_id: &i64) -> Result<User, AppError> {
        let user = self.user_repo.get_user(user_id).await?;
        Ok(user)
    }
}
