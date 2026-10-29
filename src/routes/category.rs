use crate::app_state::AppState;
use crate::db::stories::StoryRepository;
use crate::models::search::{Pagination, SearchPageQuery};
use crate::models::story::StorySummary;
use crate::templates::views::category_index_template;
use crate::utils::{current_epoch_millis, sample_key};
use actix_web::{get, web, HttpResponse, Responder};
use anyhow::Result;
use rusqlite::OptionalExtension;

async fn get_category_page(
    state: web::Data<AppState>,
    kind: &'static str,
    name: String,
    query: web::Query<SearchPageQuery>,
) -> impl Responder {
    let conn = match state.stories_pool.get().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB connection error"),
    };

    let page = query.page.unwrap_or(1).max(1);
    let page_size = 100;
    let seed = query.seed.unwrap_or_else(current_epoch_millis);
    let name_clone = name.clone();

    let result = conn
    .interact(move |conn| -> Result<(usize, Vec<StorySummary>)> {
        let (table, column, lookup) = if kind == "Tag" {
            ("story_tags", "tag_id", "tags")
        } else {
            ("story_keywords", "keyword_id", "keywords")
        };
        let lookup_sql = format!("SELECT id FROM {lookup} WHERE name = ?1 COLLATE NOCASE LIMIT 1");
        let category_id: Option<i64> = conn.query_row(&lookup_sql, [&name_clone], |r| r.get(0)).optional()?;
        let Some(category_id) = category_id else { return Ok((0, vec![])); };
        let pivot = sample_key(seed as u64);
        let mut ids = Vec::new();
        for operator in [">=", "<"] {
            let sql = format!("SELECT story_id FROM {table} WHERE {column} = ?1 AND sample_key {operator} ?2 ORDER BY sample_key, story_id LIMIT ?3");
            let mut stmt = conn.prepare(&sql)?;
            let remaining = 1001 - ids.len();
            let rows = stmt.query_map(rusqlite::params![category_id, pivot, remaining as i64], |r| r.get::<_, i64>(0))?;
            ids.extend(rows.collect::<rusqlite::Result<Vec<_>>>()?);
            if ids.len() == 1001 { break; }
        }
        let total_pool = ids.len();
        let offset = page.saturating_sub(1).saturating_mul(page_size);
        if offset >= total_pool.min(1000) { return Ok((total_pool, vec![])); }
        let end = (offset + page_size).min(total_pool.min(1000));
        let page_ids = &ids[offset..end];
        let stories = StoryRepository::fetch_by_ids(conn, page_ids)?;

        Ok((total_pool, stories))
    })
    .await;

    match result {
        Ok(Ok((total_pool, stories))) => {
            let total_pages = (total_pool.min(1000) + page_size - 1) / page_size;
            let pagination = Pagination {
                current_page: page,
                total_pages: total_pages.max(1),
                has_previous: page > 1,
                has_next: page < total_pages,
            };
            let markup = category_index_template(
                &state.base_path,
                kind,
                &name,
                total_pool,
                &stories,
                &pagination,
                seed,
                &state.popular_tags,
            );
            HttpResponse::Ok().content_type("text/html; charset=utf-8").body(markup.into_string())
        }
        _ => HttpResponse::InternalServerError().body(format!("Failed to load {}", kind.to_lowercase())),
    }
}

#[get("/tag/{tag}")]
pub async fn tag_page(
    state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<SearchPageQuery>,
) -> impl Responder {
    get_category_page(state, "Tag", path.into_inner(), query).await
}

#[get("/keyword/{keyword}")]
pub async fn keyword_page(
    state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<SearchPageQuery>,
) -> impl Responder {
    get_category_page(state, "Keyword", path.into_inner(), query).await
}
