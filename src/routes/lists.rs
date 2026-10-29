use actix_identity::Identity;
use actix_web::{get, post, web, HttpResponse, Responder};
use anyhow::Result;
use std::collections::HashMap;

use crate::app_state::AppState;
use crate::db::lists::ListRepository;
use crate::db::stories::StoryRepository;
use crate::models::list::{AddToListForm, CreateListForm, ReadingListItem};
use crate::templates::views::{reading_list_detail_template, reading_lists_page_template};
use crate::utils::url;

fn get_authenticated_user_id(base_path: &str, identity: Option<Identity>) -> Result<i64, HttpResponse> {
    match identity {
        Some(id) => match id.id() {
            Ok(user_id_str) => user_id_str
            .parse::<i64>()
            .map_err(|_| HttpResponse::Unauthorized().finish()),
            Err(_) => Err(HttpResponse::SeeOther().insert_header(("Location", url(base_path, "/login"))).finish()),
        },
        None => Err(HttpResponse::SeeOther().insert_header(("Location", url(base_path, "/login"))).finish()),
    }
}

#[get("/lists")]
pub async fn get_lists(state: web::Data<AppState>, identity: Option<Identity>) -> impl Responder {
    let user_id = match get_authenticated_user_id(&state.base_path, identity) {
        Ok(id) => id,
        Err(redirect) => return redirect,
    };
    let u_conn = match state.users_pool.get().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB connection error"),
    };
    let lists_res = u_conn
    .interact(move |conn| -> Result<_> { Ok((ListRepository::get_user_lists(conn, user_id)?, crate::db::users::UserRepository::is_admin(conn, user_id)?)) })
    .await;

    match lists_res {
        Ok(Ok((lists, is_admin))) => {
            let markup = reading_lists_page_template(&state.base_path, &lists, &state.popular_tags, is_admin);
            HttpResponse::Ok().content_type("text/html; charset=utf-8").body(markup.into_string())
        }
        _ => HttpResponse::InternalServerError().body("Failed to load lists"),
    }
}

#[post("/lists")]
pub async fn create_list(
    state: web::Data<AppState>,
    identity: Option<Identity>,
    form: web::Form<CreateListForm>,
) -> impl Responder {
    let user_id = match get_authenticated_user_id(&state.base_path, identity) {
        Ok(id) => id,
        Err(redirect) => return redirect,
    };
    let u_conn = match state.users_pool.get().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB connection error"),
    };
    let title = form.title.trim().to_string();
    let is_public = form.is_public.unwrap_or(false);

    let res = u_conn
    .interact(move |conn| ListRepository::create_list(conn, user_id, &title, is_public))
    .await;

    match res {
        Ok(Ok(list_id)) => HttpResponse::SeeOther()
        .insert_header(("Location", url(&state.base_path, &format!("/lists/{}", list_id))))
        .finish(),
        _ => HttpResponse::InternalServerError().body("Failed to create list"),
    }
}

#[get("/lists/{id}")]
pub async fn get_list_detail(
    state: web::Data<AppState>,
    path: web::Path<i64>,
    identity: Option<Identity>,
) -> impl Responder {
    let user_id = match get_authenticated_user_id(&state.base_path, identity) { Ok(id) => id, Err(response) => return response };
    let list_id = path.into_inner();
    let u_conn = match state.users_pool.get().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB connection error"),
    };

    let list_data = u_conn
    .interact(move |conn| -> Result<_> {
        let meta = ListRepository::get_list(conn, list_id)?;
        if !meta.as_ref().is_some_and(|list| list.is_public || list.user_id == user_id) { return Ok((None, vec![])); }
        let raw_items = ListRepository::get_list_items(conn, list_id)?;
        Ok((meta, raw_items))
    })
    .await;

    let (list_meta, raw_items) = match list_data {
        Ok(Ok((Some(meta), items))) => (meta, items),
        Ok(Ok((None, _))) => return HttpResponse::NotFound().body("List not found"),
        _ => return HttpResponse::InternalServerError().body("Error fetching list data"),
    };

    let story_ids: Vec<i64> = raw_items.iter().map(|(id, _, _)| *id).collect();

    let s_conn = match state.stories_pool.get().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB connection error"),
    };
    let stories = s_conn
    .interact(move |conn| StoryRepository::fetch_by_ids(conn, &story_ids))
    .await
    .unwrap_or_else(|_| Ok(vec![]))
    .unwrap_or_default();

    let story_map: HashMap<i64, _> = stories.into_iter().map(|s| (s.id, s)).collect();

    let items: Vec<ReadingListItem> = raw_items
    .into_iter()
    .filter_map(|(story_id, notes, added_at)| {
        story_map.get(&story_id).cloned().map(|story| ReadingListItem {
            story,
            notes,
            added_at,
        })
    })
    .collect();

    let markup = reading_list_detail_template(&state.base_path, &list_meta, &items, &state.popular_tags);
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(markup.into_string())
}

#[post("/lists/{id}/items")]
pub async fn add_item_to_list(
    state: web::Data<AppState>,
    path: web::Path<i64>,
    form: web::Form<AddToListForm>,
    identity: Option<Identity>,
) -> impl Responder {
    let user_id = match get_authenticated_user_id(&state.base_path, identity) { Ok(id) => id, Err(response) => return response };
    let list_id = path.into_inner();
    let u_conn = match state.users_pool.get().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB connection error"),
    };
    let story_id = form.story_id;
    let notes = form.notes.clone();

    let res = u_conn
    .interact(move |conn| ListRepository::add_story(conn, list_id, user_id, story_id, notes.as_deref()))
    .await;

    match res {
        Ok(Ok(true)) => HttpResponse::SeeOther()
        .insert_header(("Location", url(&state.base_path, &format!("/lists/{}", list_id))))
        .finish(),
        Ok(Ok(false)) => HttpResponse::NotFound().finish(),
        _ => HttpResponse::InternalServerError().body("Failed to add story to list"),
    }
}

#[post("/lists/{id}/items/{story_id}/delete")]
pub async fn remove_item_from_list(
    state: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
    identity: Option<Identity>,
) -> impl Responder {
    let user_id = match get_authenticated_user_id(&state.base_path, identity) { Ok(id) => id, Err(response) => return response };
    let (list_id, story_id) = path.into_inner();
    let u_conn = match state.users_pool.get().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB connection error"),
    };

    let res = u_conn
    .interact(move |conn| ListRepository::remove_story(conn, list_id, user_id, story_id))
    .await;

    match res {
        Ok(Ok(true)) => HttpResponse::SeeOther()
        .insert_header(("Location", url(&state.base_path, &format!("/lists/{}", list_id))))
        .finish(),
        Ok(Ok(false)) => HttpResponse::NotFound().finish(),
        _ => HttpResponse::InternalServerError().body("Failed to remove item"),
    }
}

#[post("/lists/{id}/delete")]
pub async fn delete_list(
    state: web::Data<AppState>,
    identity: Option<Identity>,
    path: web::Path<i64>,
) -> impl Responder {
    let user_id = match get_authenticated_user_id(&state.base_path, identity) {
        Ok(id) => id,
        Err(redirect) => return redirect,
    };
    let list_id = path.into_inner();
    let u_conn = match state.users_pool.get().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB connection error"),
    };

    let res = u_conn
    .interact(move |conn| ListRepository::delete_list(conn, list_id, user_id))
    .await;

    match res {
        Ok(Ok(_)) => HttpResponse::SeeOther()
        .insert_header(("Location", url(&state.base_path, "/lists")))
        .finish(),
        _ => HttpResponse::InternalServerError().body("Failed to delete list"),
    }
}
