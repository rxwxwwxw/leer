use crate::models::story::StorySummary;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchScope {
    All,
    Title,
    Author,
    Content,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchSort {
    Relevance,
    Newest,
    Oldest,
    Title,
}

#[derive(Clone, Debug)]
pub struct AuthorSearchResult {
    pub name: String,
    pub story_count: i64,
}

#[derive(Clone, Debug)]
pub enum SearchResultPayload {
    Stories(Vec<StorySummary>),
    Authors(Vec<AuthorSearchResult>),
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct SearchPageQuery {
    pub query: Option<String>,
    pub page: Option<usize>,
    pub scope: Option<SearchScope>,
    pub sort: Option<SearchSort>,
    pub seed: Option<i64>,
    pub and_tags: Option<String>,
    pub or_tags: Option<String>,
    pub exc_tags: Option<String>,
    pub inc_kws: Option<String>,
    pub exc_kws: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Pagination {
    pub current_page: usize,
    pub total_pages: usize,
    pub has_previous: bool,
    pub has_next: bool,
}
