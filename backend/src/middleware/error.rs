use actix_web::{HttpResponse, web};

use crate::errors::AppError;

pub fn json_config() -> web::JsonConfig {
    web::JsonConfig::default().error_handler(|error, _| {
        let error = AppError::Validation(error.to_string());
        actix_web::error::InternalError::from_response(
            error.to_string(),
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": "bad_request",
                "message": error.to_string(),
            })),
        )
        .into()
    })
}
