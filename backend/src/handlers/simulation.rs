use actix_web::{HttpResponse, web};

use crate::{
    AppState,
    errors::AppError,
    middleware::auth::{AdminUser, AuthenticatedUser},
    models::simulation::{CreateSimulationRequest, UpdateSimulationConfigRequest},
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/simulations")
            .route("", web::get().to(list_simulations))
            .route("", web::post().to(create_simulation))
            .route("/{id}", web::get().to(get_simulation))
            .route("/{id}/config", web::put().to(update_config))
            .route("/{id}/submit", web::post().to(submit_simulation))
            .route("/{id}/results", web::get().to(get_results))
            .route("/{id}/status", web::get().to(get_status)),
    );

    cfg.service(
        web::scope("/api/admin")
            .route("/jobs", web::get().to(list_jobs))
            .route("/stats", web::get().to(get_stats)),
    );
}

async fn list_simulations(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    let simulations = state.simulation_service.list_simulations(user.user_id).await?;
    Ok(HttpResponse::Ok().json(simulations))
}

async fn create_simulation(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    payload: web::Json<CreateSimulationRequest>,
) -> Result<HttpResponse, AppError> {
    let simulation = state
        .simulation_service
        .create_simulation(user.user_id, payload.into_inner())
        .await?;
    Ok(HttpResponse::Created().json(simulation))
}

async fn get_simulation(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let simulation = state
        .simulation_service
        .get_simulation_detail(user.user_id, path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(simulation))
}

async fn update_config(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<i64>,
    payload: web::Json<UpdateSimulationConfigRequest>,
) -> Result<HttpResponse, AppError> {
    let config = state
        .simulation_service
        .update_config(user.user_id, path.into_inner(), payload.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(config))
}

async fn submit_simulation(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let response = state
        .simulation_service
        .submit_simulation(user.user_id, path.into_inner())
        .await?;
    Ok(HttpResponse::Accepted().json(response))
}

async fn get_results(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let results = state
        .simulation_service
        .get_results(user.user_id, path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(results))
}

async fn get_status(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let status = state
        .simulation_service
        .get_status(user.user_id, path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(status))
}

async fn list_jobs(
    state: web::Data<AppState>,
    _admin: AdminUser,
) -> Result<HttpResponse, AppError> {
    let jobs = state.simulation_service.list_jobs().await?;
    Ok(HttpResponse::Ok().json(jobs))
}

async fn get_stats(
    state: web::Data<AppState>,
    _admin: AdminUser,
) -> Result<HttpResponse, AppError> {
    let stats = state.simulation_service.get_admin_stats().await?;
    Ok(HttpResponse::Ok().json(stats))
}
