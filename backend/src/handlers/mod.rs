pub mod auth;
pub mod circuit;
pub mod simulation;
pub mod ws;

use actix_web::{HttpResponse, Responder, web};

pub fn configure(cfg: &mut web::ServiceConfig) {
    auth::configure(cfg);
    circuit::configure(cfg);
    simulation::configure(cfg);
    ws::configure(cfg);
}

pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "circuit-simulator-backend"
    }))
}
