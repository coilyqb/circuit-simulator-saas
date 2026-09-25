use std::{collections::HashMap, sync::Arc};

use serde::Serialize;
use sqlx::MySqlPool;
use tokio::sync::RwLock;

use crate::{
    errors::AppError,
    models::user::{LoginRequest, LogoutRequest, PublicUser, RefreshRequest, RegisterRequest, User},
    utils::{
        hash::{hash_password, verify_password},
        jwt::{Claims, TokenPair, issue_token_pair, validate_token},
    },
};

#[derive(Debug, Clone, Serialize)]
pub struct AuthResponse {
    pub user: PublicUser,
    #[serde(flatten)]
    pub tokens: TokenPair,
}

#[derive(Clone)]
pub struct AuthService {
    pool: MySqlPool,
    access_secret: String,
    refresh_secret: String,
    access_token_ttl_minutes: i64,
    refresh_token_ttl_days: i64,
    bcrypt_cost: u32,
    refresh_tokens: Arc<RwLock<HashMap<String, i64>>>,
}

impl AuthService {
    pub fn new(
        pool: MySqlPool,
        access_secret: String,
        refresh_secret: String,
        access_token_ttl_minutes: i64,
        refresh_token_ttl_days: i64,
        bcrypt_cost: u32,
    ) -> Self {
        Self {
            pool,
            access_secret,
            refresh_secret,
            access_token_ttl_minutes,
            refresh_token_ttl_days,
            bcrypt_cost,
            refresh_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, request: RegisterRequest) -> Result<AuthResponse, AppError> {
        validate_credentials(&request.email, &request.password)?;

        let existing = sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE email = ?")
            .bind(&request.email)
            .fetch_optional(&self.pool)
            .await?;

        if existing.is_some() {
            return Err(AppError::Conflict("email is already registered".to_owned()));
        }

        let password_hash = hash_password(&request.password, self.bcrypt_cost)?;

        let result = sqlx::query(
            r#"
            INSERT INTO users (email, password_hash)
            VALUES (?, ?)
            "#,
        )
        .bind(&request.email)
        .bind(password_hash)
        .execute(&self.pool)
        .await?;

        let user = self.get_user_by_id(result.last_insert_id() as i64).await?;
        self.issue_auth_response(user).await
    }

    pub async fn login(&self, request: LoginRequest) -> Result<AuthResponse, AppError> {
        validate_credentials(&request.email, &request.password)?;

        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, password_hash, created_at, updated_at
            FROM users
            WHERE email = ?
            "#,
        )
        .bind(&request.email)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::Unauthorized("invalid email or password".to_owned()))?;

        if !verify_password(&request.password, &user.password_hash)? {
            return Err(AppError::Unauthorized(
                "invalid email or password".to_owned(),
            ));
        }

        self.issue_auth_response(user).await
    }

    pub async fn refresh(&self, request: RefreshRequest) -> Result<AuthResponse, AppError> {
        let claims = validate_token(&request.refresh_token, &self.refresh_secret, "refresh")?;
        let mut refresh_tokens = self.refresh_tokens.write().await;

        let Some(stored_user_id) = refresh_tokens.remove(&request.refresh_token) else {
            return Err(AppError::Unauthorized(
                "refresh token is no longer valid".to_owned(),
            ));
        };

        if stored_user_id != claims.sub {
            return Err(AppError::Unauthorized(
                "refresh token subject mismatch".to_owned(),
            ));
        }

        let user = self.get_user_by_id(claims.sub).await?;
        let response = self.issue_auth_response(user).await?;
        refresh_tokens.insert(response.tokens.refresh_token.clone(), claims.sub);
        Ok(response)
    }

    pub async fn logout(&self, request: LogoutRequest) -> Result<(), AppError> {
        let mut refresh_tokens = self.refresh_tokens.write().await;
        refresh_tokens.remove(&request.refresh_token);
        Ok(())
    }

    pub fn validate_access_token(&self, token: &str) -> Result<Claims, AppError> {
        validate_token(token, &self.access_secret, "access")
    }

    async fn issue_auth_response(&self, user: User) -> Result<AuthResponse, AppError> {
        let tokens = issue_token_pair(
            user.id,
            &user.email,
            &self.access_secret,
            &self.refresh_secret,
            self.access_token_ttl_minutes,
            self.refresh_token_ttl_days,
        )?;

        let mut refresh_tokens = self.refresh_tokens.write().await;
        refresh_tokens.insert(tokens.refresh_token.clone(), user.id);

        Ok(AuthResponse {
            user: user.into(),
            tokens,
        })
    }

    async fn get_user_by_id(&self, user_id: i64) -> Result<User, AppError> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, password_hash, created_at, updated_at
            FROM users
            WHERE id = ?
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".to_owned()))
    }
}

fn validate_credentials(email: &str, password: &str) -> Result<(), AppError> {
    if !email.contains('@') {
        return Err(AppError::Validation("email must be valid".to_owned()));
    }

    if password.len() < 8 {
        return Err(AppError::Validation(
            "password must be at least 8 characters".to_owned(),
        ));
    }

    Ok(())
}
