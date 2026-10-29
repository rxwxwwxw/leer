use maud::PreEscaped;
use pulldown_cmark::{html, Options, Parser};
use serde::Deserialize;

pub const UNTITLED_STORY_LABEL: &str = "(not title)";

pub fn display_story_title(title: &str) -> &str {
    if title.trim().is_empty() {
        UNTITLED_STORY_LABEL
    } else {
        title
    }
}

#[derive(Clone, Debug)]
pub struct TagCount {
    pub name: String,
    pub count: i64,
}

#[derive(Clone, Debug)]
pub struct StorySummary {
    pub id: i64,
    pub title: String,
    pub author: String,
    pub score: Option<f32>,
    pub readers: Option<i64>,
    pub date: Option<String>,
    pub tags: Option<String>,
    pub keywords: Option<String>,
    pub introduction: Option<String>,
}

#[derive(Clone, Debug)]
pub struct StoryDetail {
    pub id: i64,
    pub title: String,
    pub author: String,
    pub datetime: String,
    pub upvotes: i64,
    pub downvotes: i64,
    pub readers: i64,
    pub tags: Option<String>,
    pub keywords: Option<String>,
    pub introduction: Option<String>,
    pub content: String,
}

#[derive(Clone, Debug)]
pub struct AuthorDirectoryEntry {
    pub name: String,
    pub story_count: i64,
    pub tagged_story_count: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorTagCoverage {
    All,
    Any,
    None,
    AllTagged,
    Mixed,
}

impl Default for AuthorTagCoverage {
    fn default() -> Self {
        Self::All
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct StoryPageQuery {
    pub raw: Option<bool>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct AuthorPageQuery {
    pub page: Option<usize>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct AuthorsPageQuery {
    pub page: Option<usize>,
    pub query: Option<String>,
    pub tag_coverage: Option<AuthorTagCoverage>,
    pub min_stories: Option<usize>,
}

pub fn render_story_body(raw: &str) -> PreEscaped<String> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    let parser = Parser::new_ext(raw, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    PreEscaped(html_output)
}
