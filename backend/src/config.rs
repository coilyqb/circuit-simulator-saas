use std::env;

use crate::errors::AppError;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub server_host: String,
    pub server_port: u16,
    pub database_url: String,
    pub jwt_access_secret: String,
    pub jwt_refresh_secret: String,
    pub access_token_ttl_minutes: i64,
    pub refresh_token_ttl_days: i64,
    pub bcrypt_cost: u32,
    pub cors_allowed_origin: String,
    pub admin_emails: Vec<String>,
    pub queue_worker_count: usize,
    pub simulation_delay_ms: u64,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        dotenv::dotenv().ok();

        let database_url = required_env("DATABASE_URL")?;
        let jwt_access_secret = required_env("JWT_ACCESS_SECRET")?;
        let jwt_refresh_secret = required_env("JWT_REFRESH_SECRET")?;

        Ok(Self {
            server_host: env_or("SERVER_HOST", "127.0.0.1"),
            server_port: parse_or("SERVER_PORT", 8080_u16)?,
            database_url,
            jwt_access_secret,
            jwt_refresh_secret,
            access_token_ttl_minutes: parse_or("ACCESS_TOKEN_TTL_MINUTES", 15_i64)?,
            refresh_token_ttl_days: parse_or("REFRESH_TOKEN_TTL_DAYS", 30_i64)?,
            bcrypt_cost: parse_or("BCRYPT_COST", 12_u32)?,
            cors_allowed_origin: env_or("CORS_ALLOWED_ORIGIN", "http://localhost:3000"),
            admin_emails: env::var("ADMIN_EMAILS")
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
            queue_worker_count: parse_or("QUEUE_WORKER_COUNT", 1_usize)?,
            simulation_delay_ms: parse_or("SIMULATION_DELAY_MS", 500_u64)?,
        })
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }

    pub fn is_admin_email(&self, email: &str) -> bool {
        self.admin_emails
            .iter()
            .any(|configured| configured.eq_ignore_ascii_case(email))
    }
}

fn required_env(key: &str) -> Result<String, AppError> {
    env::var(key).map_err(|_| AppError::Config(format!("missing required environment variable `{key}`")))
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_owned())
}

fn parse_or<T>(key: &str, default: T) -> Result<T, AppError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    env::var(key)
        .map(|value| {
            value
                .parse::<T>()
                .map_err(|error| AppError::Config(format!("invalid `{key}` value: {error}")))
        })
        .unwrap_or(Ok(default))
}

#[cfg(test)]
mod tests {
    use super::AppConfig;

    #[test]
    fn admin_email_matching_is_case_insensitive() {
        let config = AppConfig {
            server_host: "127.0.0.1".into(),
            server_port: 8080,
            database_url: "mysql://localhost/test".into(),
            jwt_access_secret: "access".into(),
            jwt_refresh_secret: "refresh".into(),
            access_token_ttl_minutes: 15,
            refresh_token_ttl_days: 30,
            bcrypt_cost: 12,
            cors_allowed_origin: "*".into(),
            admin_emails: vec!["Admin@Example.com".into()],
            queue_worker_count: 1,
            simulation_delay_ms: 10,
        };

        assert!(config.is_admin_email("admin@example.com"));
        assert!(!config.is_admin_email("user@example.com"));
    }
}
