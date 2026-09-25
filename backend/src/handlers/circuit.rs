use actix_web::{HttpResponse, web};

use crate::{
    AppState,
    errors::AppError,
    middleware::auth::AuthenticatedUser,
    models::circuit::{
        CreateCircuitRequest, CreateComponentRequest, CreateConnectionRequest, UpdateCircuitRequest,
    },
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/circuits")
            .route("", web::get().to(list_circuits))
            .route("", web::post().to(create_circuit))
            .route("/{id}", web::get().to(get_circuit))
            .route("/{id}", web::put().to(update_circuit))
            .route("/{id}", web::delete().to(delete_circuit))
            .route("/{id}/components", web::post().to(add_component))
            .route("/{id}/components/{comp_id}", web::delete().to(delete_component))
            .route("/{id}/connections", web::post().to(add_connection))
            .route("/{id}/connections/{conn_id}", web::delete().to(delete_connection)),
    );
}

async fn list_circuits(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    let circuits = state.circuit_service.list_circuits(user.user_id).await?;
    Ok(HttpResponse::Ok().json(circuits))
}

async fn create_circuit(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    payload: web::Json<CreateCircuitRequest>,
) -> Result<HttpResponse, AppError> {
    let circuit = state
        .circuit_service
        .create_circuit(user.user_id, payload.into_inner())
        .await?;
    Ok(HttpResponse::Created().json(circuit))
}

async fn get_circuit(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let circuit = state
        .circuit_service
        .get_circuit_detail(user.user_id, path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(circuit))
}

async fn update_circuit(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<i64>,
    payload: web::Json<UpdateCircuitRequest>,
) -> Result<HttpResponse, AppError> {
    let circuit = state
        .circuit_service
        .update_circuit(user.user_id, path.into_inner(), payload.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(circuit))
}

async fn delete_circuit(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    state
        .circuit_service
        .delete_circuit(user.user_id, path.into_inner())
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

async fn add_component(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<i64>,
    payload: web::Json<CreateComponentRequest>,
) -> Result<HttpResponse, AppError> {
    let component = state
        .circuit_service
        .add_component(user.user_id, path.into_inner(), payload.into_inner())
        .await?;
    Ok(HttpResponse::Created().json(component))
}

async fn delete_component(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<(i64, i64)>,
) -> Result<HttpResponse, AppError> {
    let (circuit_id, component_id) = path.into_inner();
    state
        .circuit_service
        .delete_component(user.user_id, circuit_id, component_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

async fn add_connection(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<i64>,
    payload: web::Json<CreateConnectionRequest>,
) -> Result<HttpResponse, AppError> {
    let connection = state
        .circuit_service
        .add_connection(user.user_id, path.into_inner(), payload.into_inner())
        .await?;
    Ok(HttpResponse::Created().json(connection))
}

async fn delete_connection(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<(i64, i64)>,
) -> Result<HttpResponse, AppError> {
    let (circuit_id, connection_id) = path.into_inner();
    state
        .circuit_service
        .delete_connection(user.user_id, circuit_id, connection_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}
