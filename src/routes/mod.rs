// src/routes/mod.rs
pub mod auth;
pub mod admin_users;
pub mod author;
pub mod category;
pub mod home;
pub mod lists;
pub mod search;
pub mod story;

use actix_web::web;

pub fn register_protected(cfg: &mut web::ServiceConfig) {
    cfg.service(home::index_page)
    .service(search::search_page)
    .service(story::story_page)
    .service(story::story_epub)
    .service(category::tag_page)
    .service(category::keyword_page)
    .service(author::author_page)
    .service(author::authors_page)
    .service(auth::logout)
    .service(admin_users::users_page)
    .service(admin_users::create_users)
    // Reading lists endpoints
    .service(lists::get_lists)
    .service(lists::create_list)
    .service(lists::get_list_detail)
    .service(lists::add_item_to_list)
    .service(lists::remove_item_from_list)
    .service(lists::delete_list);
}

pub fn register_public(cfg: &mut web::ServiceConfig) {
    cfg.service(auth::login_page)
        .service(auth::login_submit);
}
