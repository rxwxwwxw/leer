// src/main.rs
use actix_files::Files;
use actix_identity::IdentityMiddleware;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{cookie::Key, web, App, HttpServer};
use anyhow::Result;
use deadpool_sqlite::{Config, Runtime};
use dotenvy::dotenv;
use std::env;
use std::sync::Arc;

use NG_Reader::{
    app_state::AppState,
    db::schema::initialize_user_schema,
    db::stories::StoryRepository,
    routes,
};

#[actix_web::main]
async fn main() -> Result<()> {
    // Load local .env file if it exists (silent fallback if missing in production environments)
    let _ = dotenv();


    // 1. Dynamic Environment Configurations
    let app_env = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
    let is_production = app_env == "production";

    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT")
    .unwrap_or_else(|_| "8080".to_string())
    .parse::<u16>()
    .expect("PORT environment variable must be a valid number");

    let stories_db_path = env::var("STORIES_DB_URL").unwrap_or_else(|_| "file:stories.db?mode=ro".to_string());
    let users_db_path = env::var("USERS_DB_URL").unwrap_or_else(|_| "users.db".to_string());

    // 2. Secret key for encrypted session cookies
    // In local dev, fallback to a static string. In production, require or fetch a persistent secret key.
    let secret_key_raw = env::var("SESSION_SECRET_KEY")
    .unwrap_or_else(|_| "a_very_long_and_secure_random_string_for_dev_only_32_bytes_long_minimum".to_string());
    let secret_key = Key::from(secret_key_raw.as_bytes());

    // 3. Read-Only stories.db pool
    let stories_cfg = Config::new(&stories_db_path);
    let stories_pool = stories_cfg.create_pool(Runtime::Tokio1)?;

    // 4. Read-Write users.db pool
    let users_cfg = Config::new(&users_db_path);
    let users_pool = users_cfg.create_pool(Runtime::Tokio1)?;

    let user_conn = users_pool.get().await?;
    user_conn.interact(|conn| initialize_user_schema(conn)).await.unwrap()?;

    // 5. Initial context extraction
    let init_conn = stories_pool.get().await?;
    let (popular_tags, max_story_id) = init_conn.interact(|conn| -> Result<_> {
        let mut tags = StoryRepository::get_popular_tags(conn, 1)?;
        tags.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        let max_id: i64 = conn.query_row(
            "SELECT COALESCE(MAX(id), 1) FROM stories_metadata",
                                         [],
                                         |r| r.get(0),
        )?;
        Ok((tags, max_id))
    }).await.unwrap()?;

    // 6. Static Embeddings Model Load

    // Read and sanitize BASE_PATH (e.g. "/device2" or "")
    let raw_base = env::var("BASE_PATH").unwrap_or_default();
    let base_path = if raw_base.is_empty() || raw_base == "/" {
        "".to_string()
    } else {
        format!("/{}", raw_base.trim_matches('/'))
    };

    // Cookie name and path
    let cookie_name = if base_path.is_empty() {
        "app_session".to_string()
    } else {
        format!("{}_session", base_path.trim_matches('/').replace('/', "_"))
    };
    let cookie_path = if base_path.is_empty() {
        "/".to_string()
    } else {
        format!("{}/", base_path)
    };

    // AppState instantiation
    let app_state = web::Data::new(AppState {
        base_path: base_path.clone(),
                                   stories_pool,
                                   users_pool,
                                   popular_tags: Arc::new(popular_tags),
                                   max_story_id,
    });

    let static_path =format!("{}/static",base_path);

    HttpServer::new(move || {
        let base_scope = web::scope(&base_path)
        .service(Files::new(&static_path, "./static").prefer_utf8(true))
        .configure(routes::register);

        App::new()
        .wrap(IdentityMiddleware::default())
        .wrap(
            SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
            .cookie_name(cookie_name.clone())
            .cookie_path(cookie_path.clone())
            .cookie_secure(is_production)
            .cookie_http_only(true)
            .build(),
        )
        .app_data(app_state.clone())
        .service(base_scope)
    })
    .bind((host, port))?
    .run()
    .await?;

    Ok(())
}
