use actix_web::{HttpResponse, web};

use crate::{
    AppState,
    errors::AppError,
    models::user::{LoginRequest, LogoutRequest, RefreshRequest, RegisterRequest},
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/auth")
            .route("/register", web::post().to(register))
            .route("/login", web::post().to(login))
            .route("/refresh", web::post().to(refresh))
            .route("/logout", web::post().to(logout)),
    );
}

async fn register(
    state: web::Data<AppState>,
    payload: web::Json<RegisterRequest>,
) -> Result<HttpResponse, AppError> {
    let response = state.auth_service.register(payload.into_inner()).await?;
    Ok(HttpResponse::Created().json(response))
}

async fn login(
    state: web::Data<AppState>,
    payload: web::Json<LoginRequest>,
) -> Result<HttpResponse, AppError> {
    let response = state.auth_service.login(payload.into_inner()).await?;
    Ok(HttpResponse::Ok().json(response))
}

async fn refresh(
    state: web::Data<AppState>,
    payload: web::Json<RefreshRequest>,
) -> Result<HttpResponse, AppError> {
    let response = state.auth_service.refresh(payload.into_inner()).await?;
    Ok(HttpResponse::Ok().json(response))
}

async fn logout(
    state: web::Data<AppState>,
    payload: web::Json<LogoutRequest>,
) -> Result<HttpResponse, AppError> {
    state.auth_service.logout(payload.into_inner()).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({ "message": "logged out" })))
}
