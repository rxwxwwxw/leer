use actix_identity::Identity;
use actix_web::{body::{BoxBody, MessageBody}, dev::{ServiceRequest, ServiceResponse}, middleware::Next, web, FromRequest, HttpResponse};
use crate::{app_state::AppState, db::users::UserRepository, utils::url};

/// Applies to every application route except login and static assets.
pub async fn require_login(
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<BoxBody>, actix_web::Error> {
    let identity = Identity::from_request(req.request(), &mut actix_web::dev::Payload::None).await.ok();
    let user_id = identity.as_ref().and_then(|id| id.id().ok()).and_then(|id| id.parse::<i64>().ok());
    let state = req.app_data::<web::Data<AppState>>().cloned();
    let authenticated = if let (Some(user_id), Some(state)) = (user_id, &state) {
        match state.users_pool.get().await {
            Ok(conn) => match conn.interact(move |conn| UserRepository::get_by_id(conn, user_id)).await {
                Ok(Ok(user)) => user.is_some(),
                _ => return Ok(req.into_response(HttpResponse::ServiceUnavailable().body("Authentication unavailable").map_into_boxed_body())),
            },
            Err(_) => return Ok(req.into_response(HttpResponse::ServiceUnavailable().body("Authentication unavailable").map_into_boxed_body())),
        }
    } else { false };
    if !authenticated {
        if let Some(identity) = identity { identity.logout(); }
        let response = if req.method() == actix_web::http::Method::GET || req.method() == actix_web::http::Method::HEAD {
            HttpResponse::SeeOther().insert_header(("Location", url(state.as_ref().map(|s| s.base_path.as_str()).unwrap_or(""), "/login"))).finish()
        } else {
            HttpResponse::Unauthorized().finish()
        };
        return Ok(req.into_response(response.map_into_boxed_body()));
    }
    Ok(next.call(req).await?.map_into_boxed_body())
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_identity::IdentityMiddleware;
    use actix_session::{storage::CookieSessionStore, SessionMiddleware};
    use actix_web::{cookie::Key, test, App};

    #[actix_web::test]
    async fn anonymous_requests_are_denied_and_redirected() {
        let app = test::init_service(App::new()
            .wrap(IdentityMiddleware::default())
            .wrap(SessionMiddleware::new(CookieSessionStore::default(), Key::generate()))
            .service(web::scope("").wrap(actix_web::middleware::from_fn(require_login))
                .route("/story/1", web::get().to(|| async { HttpResponse::Ok().body("secret") }))
                .route("/lists", web::post().to(|| async { HttpResponse::Ok() })))).await;
        let response = test::call_service(&app, test::TestRequest::get().uri("/story/1").to_request()).await;
        assert_eq!(response.status(), actix_web::http::StatusCode::SEE_OTHER);
        assert_eq!(response.headers().get("Location").unwrap(), "/login");
        let response = test::call_service(&app, test::TestRequest::post().uri("/lists").to_request()).await;
        assert_eq!(response.status(), actix_web::http::StatusCode::UNAUTHORIZED);
    }
}
