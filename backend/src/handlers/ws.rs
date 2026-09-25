use actix_web::{HttpRequest, HttpResponse, web};
use futures_util::StreamExt;
use tokio::select;

use crate::{
    AppState,
    errors::AppError,
    middleware::auth::AuthenticatedUser,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/ws/simulations/{simulation_id}", web::get().to(simulation_updates));
}

async fn simulation_updates(
    request: HttpRequest,
    payload: web::Payload,
    path: web::Path<i64>,
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    let simulation_id = path.into_inner();

    if !state
        .simulation_service
        .simulation_belongs_to_user(user.user_id, simulation_id)
        .await?
        && !state.config.is_admin_email(&user.email)
    {
        return Err(AppError::Forbidden(
            "cannot subscribe to another user's simulation".to_owned(),
        ));
    }

    let (response, mut session, mut message_stream) = actix_ws::handle(&request, payload)?;
    let mut receiver = state.notification_service.subscribe(simulation_id).await;

    actix_web::rt::spawn(async move {
        loop {
            select! {
                maybe_message = message_stream.next() => {
                    match maybe_message {
                        Some(Ok(actix_ws::Message::Ping(bytes))) => {
                            if session.pong(&bytes).await.is_err() {
                                break;
                            }
                        }
                        Some(Ok(actix_ws::Message::Close(reason))) => {
                            let _ = session.close(reason).await;
                            break;
                        }
                        Some(Ok(actix_ws::Message::Text(text))) => {
                            if text == "ping" && session.text("pong").await.is_err() {
                                break;
                            }
                        }
                        Some(Ok(_)) => {}
                        Some(Err(_)) | None => break,
                    }
                }
                message = receiver.recv() => {
                    match message {
                        Ok(message) => {
                            if session.text(message).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        }
    });

    Ok(response)
}
