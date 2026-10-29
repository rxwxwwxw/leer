// src/routes/search.rs
use crate::app_state::AppState;
use crate::models::search::{Pagination, SearchPageQuery, SearchScope, SearchSort};
use crate::search::execute_search;
use crate::templates::views::{search_query_template, search_results_template};
use actix_web::{get, web, HttpResponse, Responder};

#[get("/search")]
pub async fn search_page(
    state: web::Data<AppState>,
    query: web::Query<SearchPageQuery>,
) -> impl Responder {
    let has_query = query.query.as_ref().map(|q| !q.trim().is_empty()).unwrap_or(false);
    let has_tags = query.and_tags.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false)
    || query.or_tags.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false)
    || query.exc_tags.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
    let has_kws = query.inc_kws.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false)
    || query.exc_kws.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);

    let has_nondefault_options = query.scope.unwrap_or(SearchScope::All) != SearchScope::All
        || query.sort.unwrap_or(SearchSort::Relevance) != SearchSort::Relevance;

    if !has_query && !has_tags && !has_kws && !has_nondefault_options {
        return HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(search_query_template(&state.base_path, &state.popular_tags, &query).into_string());
    }

    let page = query.page.unwrap_or(1).max(1);
    let page_size = 25;

    match execute_search(&state, &query, page, page_size).await {
        Ok((payload, total_count)) => {
            let total_pages = (total_count + page_size - 1) / page_size;
            let pagination = Pagination {
                current_page: page,
                total_pages: total_pages.max(1),
                has_previous: page > 1,
                has_next: page < total_pages,
            };
            let markup = search_results_template(&state.base_path, &query, &payload, total_count, &pagination, &state.popular_tags);
            HttpResponse::Ok().content_type("text/html; charset=utf-8").body(markup.into_string())
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Search failed: {}", e)),
    }
}
