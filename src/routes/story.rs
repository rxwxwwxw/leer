use crate::app_state::AppState;
use crate::db::stories::StoryRepository;
use crate::models::story::{display_story_title, StoryDetail, StoryPageQuery, StorySummary};
use crate::templates::views::story_detail_template;
use actix_web::{get, web, HttpResponse, Responder};
use anyhow::Result;

#[get("/story/{id}")]
pub async fn story_page(
    state: web::Data<AppState>,
    path: web::Path<i64>,
    query: web::Query<StoryPageQuery>,
) -> impl Responder {
    let story_id = path.into_inner();
    let is_raw = query.raw.unwrap_or(false);

    let conn = match state.stories_pool.get().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB connection error"),
    };

    let result = conn
    .interact(move |conn| -> Result<Option<(StoryDetail, Vec<StorySummary>)>> {
        let (detail, related_ids) = match StoryRepository::get_detail(conn, story_id)? {
            Some(res) => res,
              None => return Ok(None),
        };
        let ids: Vec<i64> = related_ids.iter().map(|(id, _)| *id).collect();
        let scores: std::collections::HashMap<i64, f32> = related_ids.into_iter().collect();
        let mut related = StoryRepository::fetch_by_ids(conn, &ids)?;
        for story in &mut related { story.score = scores.get(&story.id).copied(); }
        related.sort_by(|a, b| {
            display_story_title(&a.title)
                .to_lowercase()
                .cmp(&display_story_title(&b.title).to_lowercase())
                .then_with(|| a.author.to_lowercase().cmp(&b.author.to_lowercase()))
                .then_with(|| a.id.cmp(&b.id))
        });
        Ok(Some((detail, related)))
    })
    .await;

    match result {
        Ok(Ok(Some((story, related)))) => {
            let markup = story_detail_template(&state.base_path, &story, &related, is_raw);
            HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(markup.into_string())
        }
        Ok(Ok(None)) => HttpResponse::NotFound().body("Story not found"),
        _ => HttpResponse::InternalServerError().body("Failed to load story"),
    }
}

#[get("/story/{id}/epub")]
pub async fn story_epub(state: web::Data<AppState>, path: web::Path<i64>) -> impl Responder {
    let story_id = path.into_inner();
    let conn = match state.stories_pool.get().await {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("DB connection error"),
    };
    let result = conn.interact(move |conn| -> Result<Option<(Vec<u8>, String)>> {
        let Some((story, _)) = StoryRepository::get_detail(conn, story_id)? else { return Ok(None); };
        let filename = crate::epub::download_filename(&story.author, &story.title);
        Ok(Some((crate::epub::create_epub(&story)?, filename)))
    }).await;
    match result {
        Ok(Ok(Some((bytes, filename)))) => HttpResponse::Ok()
            .content_type("application/epub+zip")
            .insert_header(("Content-Disposition", format!("attachment; filename=\"story-{}.epub\"; filename*=UTF-8''{}", story_id, urlencoding::encode(&filename))))
            .body(bytes),
        Ok(Ok(None)) => HttpResponse::NotFound().body("Story not found"),
        _ => HttpResponse::InternalServerError().body("Failed to create EPUB"),
    }
}
