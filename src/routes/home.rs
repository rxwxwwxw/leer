use crate::app_state::AppState;
use crate::db::stories::StoryRepository;
use crate::templates::views::home_page_template;
use crate::utils::{current_epoch_millis, sample_random_ids, shuffle_slice};
use actix_web::{get, web, HttpResponse, Responder};
use anyhow::Result;

#[get("/")]
pub async fn index_page(state: web::Data<AppState>) -> impl Responder {
    let conn = match state.stories_pool.get().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB connection error"),
    };

    let popular_tags = state.popular_tags.clone();
    let max_id = state.max_story_id;

    let res = conn
    .interact(move |conn| -> Result<_> {
        let featured_tag = if !popular_tags.is_empty() {
            let idx = (current_epoch_millis() as usize) % popular_tags.len();
            popular_tags[idx].name.clone()
        } else {
            "Action".to_string()
        };

        let mut tag_stmt = conn.prepare(
            "SELECT st.story_id
            FROM story_tags st
            JOIN tags t ON st.tag_id = t.id
            WHERE t.name = ?1
            LIMIT 500",
        )?;

        let mut tag_story_ids: Vec<i64> = tag_stmt
        .query_map([&featured_tag], |r| r.get(0))?
        .filter_map(Result::ok)
        .collect();

        shuffle_slice(&mut tag_story_ids);
        tag_story_ids.truncate(50);
        let featured_stories = StoryRepository::fetch_by_ids(conn, &tag_story_ids)?;

        let general_random_ids = sample_random_ids(max_id, 65);
        let mut random_stories = StoryRepository::fetch_by_ids(conn, &general_random_ids)?;
        random_stories.truncate(50);

        Ok((featured_tag, featured_stories, random_stories))
    })
    .await;

    match res {
        Ok(Ok((tag_name, feat, rand_stories))) => {
            let markup = home_page_template(&state.base_path, "Tag", &tag_name, &feat, &rand_stories, &state.popular_tags);
            HttpResponse::Ok().content_type("text/html; charset=utf-8").body(markup.into_string())
        }
        _ => HttpResponse::InternalServerError().body("Failed to load home page"),
    }
}
