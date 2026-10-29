use deadpool_sqlite::Pool;
use std::sync::Arc;
use crate::models::story::TagCount;


pub struct AppState {
    pub base_path: String,
    pub stories_pool: Pool,
    pub users_pool: Pool,
    pub popular_tags: Arc<Vec<TagCount>>,
    pub max_story_id: i64,
}
