use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64,
    pub email: String,
    pub token_type: String,
    pub exp: usize,
    pub iat: usize,
    pub jti: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
    pub refresh_expires_in: i64,
}

pub fn issue_token_pair(
    user_id: i64,
    email: &str,
    access_secret: &str,
    refresh_secret: &str,
    access_token_ttl_minutes: i64,
    refresh_token_ttl_days: i64,
) -> Result<TokenPair, AppError> {
    let access_token = create_token(
        user_id,
        email,
        access_secret,
        Duration::minutes(access_token_ttl_minutes),
        "access",
    )?;
    let refresh_token = create_token(
        user_id,
        email,
        refresh_secret,
        Duration::days(refresh_token_ttl_days),
        "refresh",
    )?;

    Ok(TokenPair {
        access_token,
        refresh_token,
        token_type: "Bearer",
        expires_in: access_token_ttl_minutes * 60,
        refresh_expires_in: refresh_token_ttl_days * 24 * 60 * 60,
    })
}

pub fn validate_token(
    token: &str,
    secret: &str,
    expected_token_type: &str,
) -> Result<Claims, AppError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;

    if data.claims.token_type != expected_token_type {
        return Err(AppError::Unauthorized(format!(
            "expected `{expected_token_type}` token"
        )));
    }

    Ok(data.claims)
}

fn create_token(
    user_id: i64,
    email: &str,
    secret: &str,
    expires_in: Duration,
    token_type: &str,
) -> Result<String, AppError> {
    let issued_at = Utc::now();
    let claims = Claims {
        sub: user_id,
        email: email.to_owned(),
        token_type: token_type.to_owned(),
        exp: (issued_at + expires_in).timestamp() as usize,
        iat: issued_at.timestamp() as usize,
        jti: Uuid::new_v4().to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use super::{issue_token_pair, validate_token};

    #[test]
    fn issues_and_validates_access_and_refresh_tokens() {
        let tokens = issue_token_pair(42, "user@example.com", "access", "refresh", 15, 30)
            .expect("issue token pair");

        let access_claims =
            validate_token(&tokens.access_token, "access", "access").expect("access claims");
        let refresh_claims = validate_token(&tokens.refresh_token, "refresh", "refresh")
            .expect("refresh claims");

        assert_eq!(access_claims.sub, 42);
        assert_eq!(refresh_claims.email, "user@example.com");
        assert_ne!(tokens.access_token, tokens.refresh_token);
    }
}
