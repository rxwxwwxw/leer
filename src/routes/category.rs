use crate::app_state::AppState;
use crate::db::stories::StoryRepository;
use crate::models::search::{Pagination, SearchPageQuery};
use crate::models::story::StorySummary;
use crate::templates::views::category_index_template;
use crate::utils::{current_epoch_millis, shuffle_slice_seeded};
use actix_web::{get, web, HttpResponse, Responder};
use anyhow::Result;

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
    let page_size = 50;
    let seed = query.seed.unwrap_or_else(current_epoch_millis);
    let name_clone = name.clone();

    let result = conn
    .interact(move |conn| -> Result<(usize, Vec<StorySummary>)> {
        let id_query = if kind == "Tag" {
            "SELECT st.story_id FROM story_tags st JOIN tags t ON st.tag_id = t.id WHERE t.name = ?1 LIMIT 500"
        } else {
            "SELECT sk.story_id FROM story_keywords sk JOIN keywords k ON sk.keyword_id = k.id WHERE k.name = ?1 LIMIT 500"
        };

        let mut stmt = conn.prepare(id_query)?;
        let mut ids: Vec<i64> = stmt
        .query_map([&name_clone], |r| r.get(0))?
        .filter_map(Result::ok)
        .collect();

        let total_pool = ids.len();
        if total_pool == 0 {
            return Ok((0, vec![]));
        }

        shuffle_slice_seeded(&mut ids, (seed as u64) ^ 0x5DEECE66D);

        let offset = (page - 1) * page_size;
        if offset >= ids.len() {
            return Ok((total_pool, vec![]));
        }

        let end = (offset + page_size).min(ids.len());
        let page_ids = &ids[offset..end];
        let stories = StoryRepository::fetch_by_ids(conn, page_ids)?;

        Ok((total_pool, stories))
    })
    .await;

    match result {
        Ok(Ok((total_pool, stories))) => {
            let total_pages = (total_pool + page_size - 1) / page_size;
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
