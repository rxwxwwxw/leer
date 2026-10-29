use actix_identity::Identity;
use actix_web::{get, post, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use crate::app_state::AppState;
use crate::db::users::{hash_password, verify_password, UserRepository};
use crate::models::user::{LoginForm, RegisterForm};
use crate::templates::views::{login_page_template, register_page_template};
use crate::utils::url;

#[get("/login")]
pub async fn login_page(state: web::Data<AppState>, identity: Option<Identity>) -> impl Responder {
    if identity.is_some() {
        return HttpResponse::SeeOther()
        .insert_header(("Location", url(&state.base_path, "/lists")))
        .finish();
    }
    let markup = login_page_template(&state.base_path, None, &state.popular_tags);
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
        let markup = login_page_template(&state.base_path, Some("Invalid username or password."), &state.popular_tags);
        return HttpResponse::Unauthorized().content_type("text/html; charset=utf-8").body(markup.into_string());
    }

    if Identity::login(&req.extensions(), user_id.to_string()).is_err() {
        return HttpResponse::InternalServerError().body("Session error");
    }

    HttpResponse::SeeOther()
    .insert_header(("Location", url(&state.base_path, "/lists")))
    .finish()
}

#[get("/register")]
pub async fn register_page(state: web::Data<AppState>, identity: Option<Identity>) -> impl Responder {
    if identity.is_some() {
        return HttpResponse::SeeOther()
        .insert_header(("Location", url(&state.base_path, "/lists")))
        .finish();
    }
    let markup = register_page_template(&state.base_path, None, &state.popular_tags);
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(markup.into_string())
}

#[post("/register")]
pub async fn register_submit(
    req: HttpRequest,
    state: web::Data<AppState>,
    form: web::Form<RegisterForm>,
) -> impl Responder {
    let username = form.username.trim().to_string();
    if username.len() < 3 || username.len() > 32 {
        let markup = register_page_template(&state.base_path, Some("Username must be between 3 and 32 characters."), &state.popular_tags);
        return HttpResponse::BadRequest().content_type("text/html; charset=utf-8").body(markup.into_string());
    }
    if form.password.len() < 8 {
        let markup = register_page_template(&state.base_path, Some("Password must be at least 8 characters long."), &state.popular_tags);
        return HttpResponse::BadRequest().content_type("text/html; charset=utf-8").body(markup.into_string());
    }
    if form.password != form.confirm_password {
        let markup = register_page_template(&state.base_path, Some("Passwords do not match."), &state.popular_tags);
        return HttpResponse::BadRequest().content_type("text/html; charset=utf-8").body(markup.into_string());
    }

    let password_hash = match hash_password(&form.password) {
        Ok(h) => h,
        Err(_) => return HttpResponse::InternalServerError().body("Password processing error"),
    };

    let u_conn = match state.users_pool.get().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB connection error"),
    };

    let create_res = u_conn
    .interact(move |conn| UserRepository::create_user(conn, &username, &password_hash))
    .await;

    match create_res {
        Ok(Ok(user_id)) => {
            let _ = Identity::login(&req.extensions(), user_id.to_string());
            HttpResponse::SeeOther()
            .insert_header(("Location", url(&state.base_path, "/lists")))
            .finish()
        }
        Ok(Err(_)) => {
            let markup = register_page_template(&state.base_path, Some("Username is already taken."), &state.popular_tags);
            HttpResponse::Conflict().content_type("text/html; charset=utf-8").body(markup.into_string())
        }
        _ => HttpResponse::InternalServerError().body("Registration failed"),
    }
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
