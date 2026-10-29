use serde::Deserialize;
use crate::models::story::StorySummary;

#[derive(Clone, Debug)]
pub struct ReadingList {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub is_public: bool,
    pub created_at: i64,
    pub story_count: i64,
}

#[derive(Clone, Debug)]
pub struct ReadingListItem {
    pub story: StorySummary,
    pub notes: Option<String>,
    pub added_at: i64,
}

#[derive(Deserialize, Debug)]
pub struct CreateListForm {
    pub title: String,
    pub is_public: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct AddToListForm {
    pub story_id: i64,
    pub notes: Option<String>,
}
