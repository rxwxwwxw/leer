use actix_identity::Identity;
use actix_web::{get, post, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use crate::app_state::AppState;
use crate::db::users::{verify_password, UserRepository};
use crate::models::user::LoginForm;
use crate::templates::views::login_page_template;
use crate::utils::url;

#[get("/login")]
pub async fn login_page(state: web::Data<AppState>, identity: Option<Identity>) -> impl Responder {
    if identity.is_some() {
        return HttpResponse::SeeOther()
        .insert_header(("Location", url(&state.base_path, "/lists")))
        .finish();
    }
    let markup = login_page_template(&state.base_path, None);
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(markup.into_string())
}

#[post("/login")]
pub async fn login_submit(
    req: HttpRequest,
    state: web::Data<AppState>,
    form: web::Form<LoginForm>,
) -> impl Responder {
    let username = form.username.trim().to_string();
    let password = form.password.clone();
    let u_conn = match state.users_pool.get().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB connection error"),
    };

    let user_record = u_conn
    .interact(move |conn| UserRepository::get_by_username(conn, &username))
    .await;

    let (user_id, valid) = match user_record {
        Ok(Ok(Some((id, _, hash)))) => (id, verify_password(&password, &hash)),
        _ => (0, false),
    };

    if !valid {
        let markup = login_page_template(&state.base_path, Some("Invalid username or password."));
        return HttpResponse::Unauthorized().content_type("text/html; charset=utf-8").body(markup.into_string());
    }

    if Identity::login(&req.extensions(), user_id.to_string()).is_err() {
        return HttpResponse::InternalServerError().body("Session error");
    }

    HttpResponse::SeeOther()
    .insert_header(("Location", url(&state.base_path, "/lists")))
    .finish()
}

#[post("/logout")]
pub async fn logout(state: web::Data<AppState>, identity: Option<Identity>) -> impl Responder {
    if let Some(id) = identity {
        id.logout();
    }
    HttpResponse::SeeOther()
    .insert_header(("Location", url(&state.base_path, "/")))
    .finish()
}
