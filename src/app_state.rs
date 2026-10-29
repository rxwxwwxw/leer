use deadpool_sqlite::Pool;
use model2vec_rs::model::StaticModel;
use std::sync::Arc;
use crate::models::story::TagCount;
use crate::embeddings::abtt::AbttTransformer;


pub struct AppState {
    pub base_path: String,
    pub stories_pool: Pool,
    pub users_pool: Pool,
    pub model: Arc<StaticModel>,
    pub abtt: AbttTransformer,
    pub popular_tags: Arc<Vec<TagCount>>,
    pub max_story_id: i64,
}
