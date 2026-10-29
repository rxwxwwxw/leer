use actix_identity::Identity;
use actix_session::Session;
use actix_web::{get, post, web, HttpResponse, Responder};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use maud::{html, Markup};
use serde::Deserialize;
use std::collections::HashSet;
use crate::{app_state::AppState, db::users::{hash_password, UserRepository}, templates::layout::base_layout, utils::url};

#[derive(Deserialize)]
struct CreateUsersForm {
    csrf: String,
    username: Option<String>,
    password: Option<String>,
    bulk: Option<String>,
}

async fn authorized(state: &AppState, identity: &Identity) -> Result<bool, ()> {
    let id = identity.id().ok().and_then(|v| v.parse::<i64>().ok()).ok_or(())?;
    let conn = state.users_pool.get().await.map_err(|_| ())?;
    conn.interact(move |conn| UserRepository::is_admin(conn, id)).await.map_err(|_| ())?.map_err(|_| ())
}

fn page(base: &str, token: &str, message: Option<&str>) -> Markup {
    let heading = html! { h1 { "Manage users" } };
    let content = html! {
        @if let Some(message) = message { p { (message) } }
        h2 { "Create user" }
        form method="post" action=(url(base, "/admin/users")) {
            input type="hidden" name="csrf" value=(token);
            label { "Username " input name="username" required minlength="3" maxlength="32"; }
            label { "Password " input type="password" name="password" required minlength="8" autocomplete="new-password"; }
            button type="submit" { "Create user" }
        }
        h2 { "Create users in bulk" }
        p { "Enter one username,password pair per line, up to 50 users. Passwords are set exactly as entered." }
        form method="post" action=(url(base, "/admin/users")) {
            input type="hidden" name="csrf" value=(token);
            label { "Users" textarea name="bulk" required rows="10" {} }
            button type="submit" { "Create users" }
        }
    };
    base_layout(base, "Manage users", None, None, heading, content, None, None, None)
}

#[get("/admin/users")]
pub async fn users_page(state: web::Data<AppState>, identity: Identity, session: Session) -> impl Responder {
    match authorized(&state, &identity).await {
        Ok(true) => {}, Ok(false) => return HttpResponse::Forbidden().finish(), Err(_) => return HttpResponse::ServiceUnavailable().finish(),
    }
    let token = SaltString::generate(&mut OsRng).to_string();
    if session.insert("admin_user_csrf", &token).is_err() { return HttpResponse::InternalServerError().finish(); }
    HttpResponse::Ok().insert_header(("Cache-Control", "no-store")).content_type("text/html; charset=utf-8").body(page(&state.base_path, &token, None).into_string())
}

#[post("/admin/users")]
pub async fn create_users(state: web::Data<AppState>, identity: Identity, session: Session, form: web::Form<CreateUsersForm>) -> impl Responder {
    match authorized(&state, &identity).await {
        Ok(true) => {}, Ok(false) => return HttpResponse::Forbidden().finish(), Err(_) => return HttpResponse::ServiceUnavailable().finish(),
    }
    let token = session.get::<String>("admin_user_csrf").ok().flatten().unwrap_or_default();
    // Fixed-length tokens; compare every byte without early exit.
    if token.is_empty() || token.len() != form.csrf.len() || token.bytes().zip(form.csrf.bytes()).fold(0u8, |v, (a,b)| v | (a ^ b)) != 0 {
        return HttpResponse::Forbidden().finish();
    }
    let pairs: Vec<(String, String)> = if let Some(bulk) = &form.bulk {
        let mut items = Vec::new();
        for line in bulk.lines().filter(|line| !line.trim().is_empty()) {
            let Some((username, password)) = line.split_once(',') else { return HttpResponse::BadRequest().body("Each line must contain username,password; no accounts were created."); };
            items.push((username.trim().to_string(), password.to_string()));
        }
        items
    } else {
        vec![(form.username.as_deref().unwrap_or("").trim().to_string(), form.password.clone().unwrap_or_default())]
    };
    let mut seen = HashSet::new();
    if pairs.is_empty() || pairs.len() > 50 || pairs.iter().any(|(u,p)| !(3..=32).contains(&u.len()) || u.chars().any(|c| c.is_control() || c == ',') || p.len() < 8 || p.len() > 1024 || !seen.insert(u.to_ascii_lowercase())) {
        return HttpResponse::BadRequest().body("Invalid or duplicate username, invalid password, or more than 50 users; no accounts were created.");
    }
    let conn = match state.users_pool.get().await { Ok(conn) => conn, Err(_) => return HttpResponse::ServiceUnavailable().finish() };
    let count = pairs.len();
    let actor_id = match identity.id().ok().and_then(|v| v.parse::<i64>().ok()) { Some(id) => id, None => return HttpResponse::Forbidden().finish() };
    let result = conn.interact(move |conn| -> anyhow::Result<()> {
        let mut users = Vec::with_capacity(pairs.len());
        for (name, password) in pairs { users.push((name, hash_password(&password)?)); }
        let tx = conn.transaction()?;
        anyhow::ensure!(UserRepository::is_admin(&tx, actor_id)?, "Admin access revoked");
        for (name, hash) in users { UserRepository::create_user(&tx, &name, &hash)?; }
        tx.commit()?;
        Ok(())
    }).await;
    match result {
        Ok(Ok(())) => HttpResponse::Ok().insert_header(("Cache-Control", "no-store")).content_type("text/html; charset=utf-8").body(page(&state.base_path, &token, Some(&format!("Created {count} user(s)."))).into_string()),
        Ok(Err(_)) => HttpResponse::Conflict().body("Could not create users; a username may already exist. No accounts were created."),
        Err(_) => HttpResponse::ServiceUnavailable().finish(),
    }
}
