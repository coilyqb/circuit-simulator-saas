use std::future::Future;
use std::pin::Pin;

use actix_web::{
    FromRequest, HttpRequest, dev::Payload, error::ErrorUnauthorized, http::header, web,
};

use crate::{AppState, errors::AppError};

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: i64,
    pub email: String,
}

#[derive(Debug, Clone)]
pub struct AdminUser(pub AuthenticatedUser);

impl FromRequest for AuthenticatedUser {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let auth_header = req.headers().get(header::AUTHORIZATION).cloned();
        let state = req.app_data::<web::Data<AppState>>().cloned();

        Box::pin(async move {
            let state = state.ok_or_else(|| {
                ErrorUnauthorized(AppError::Internal("application state unavailable".to_owned()))
            })?;

            let header_value = auth_header.ok_or_else(|| {
                ErrorUnauthorized(AppError::Unauthorized(
                    "missing authorization header".to_owned(),
                ))
            })?;

            let token = header_value
                .to_str()
                .ok()
                .and_then(|value| value.strip_prefix("Bearer "))
                .ok_or_else(|| {
                    ErrorUnauthorized(AppError::Unauthorized(
                        "authorization header must use ******".to_owned(),
                    ))
                })?;

            let claims = state.auth_service.validate_access_token(token).map_err(|error| {
                ErrorUnauthorized(AppError::Unauthorized(error.to_string()))
            })?;

            Ok(Self {
                user_id: claims.sub,
                email: claims.email,
            })
        })
    }
}

impl FromRequest for AdminUser {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let auth_header = req.headers().get(header::AUTHORIZATION).cloned();
        let state = req.app_data::<web::Data<AppState>>().cloned();

        Box::pin(async move {
            let state = state.ok_or_else(|| {
                ErrorUnauthorized(AppError::Internal("application state unavailable".to_owned()))
            })?;

            let header_value = auth_header.ok_or_else(|| {
                ErrorUnauthorized(AppError::Unauthorized(
                    "missing authorization header".to_owned(),
                ))
            })?;

            let token = header_value
                .to_str()
                .ok()
                .and_then(|value| value.strip_prefix("Bearer "))
                .ok_or_else(|| {
                    ErrorUnauthorized(AppError::Unauthorized(
                        "authorization header must use ******".to_owned(),
                    ))
                })?;

            let claims = state.auth_service.validate_access_token(token).map_err(|error| {
                ErrorUnauthorized(AppError::Unauthorized(error.to_string()))
            })?;

            if !state.config.is_admin_email(&claims.email) {
                return Err(ErrorUnauthorized(AppError::Forbidden(
                    "admin access required".to_owned(),
                )));
            }

            Ok(Self(AuthenticatedUser {
                user_id: claims.sub,
                email: claims.email,
            }))
        })
    }
}
