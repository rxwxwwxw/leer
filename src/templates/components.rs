use crate::models::search::Pagination;
use crate::models::story::{display_story_title, StorySummary, TagCount};
use maud::{html, Markup};

pub fn related_story_card(base_path: &str, story: &StorySummary) -> Markup {
    let author_encoded = urlencoding::encode(&story.author);
    let display_title = display_story_title(&story.title);
    html! {
        li class="related-card" {
            h4 class="related-title" {
                a href=(format!("{}/story/{}", base_path, story.id)) { (display_title) }
            }
            p class="related-byline" {
                "by "
                a href=(format!("{}/author/{}/", base_path, author_encoded)) { (story.author) }
                @if let Some(readers) = story.readers {
                    span class="related-meta" { " · " (readers) " readers" }
                }
            }
            @if let Some(tags) = &story.tags {
                div class="pills-container" {
                    @for tag in tags.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).take(3) {
                        a class="tag-pill" href=(format!("{}/tag/{}", base_path,urlencoding::encode(tag))) { (tag) }
                    }
                }
            }
            @if let Some(keywords) = &story.keywords {
                div class="pills-container" {
                    @for kw in keywords.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).take(3) {
                        a class="kw-pill" href=(format!("{}/keyword/{}", base_path, urlencoding::encode(kw))) { (kw) }
                    }
                }
            }
            @if let Some(intro) = &story.introduction {
                @if !intro.trim().is_empty() {
                    p class="related-intro" { (intro) }
                }
            }
        }
    }
}

pub fn related_stories_section(base_path: &str, related: &[StorySummary]) -> Markup {
    if related.is_empty() {
        return html! {};
    }
    html! {
        section class="related-stories-section" aria-labelledby="related-stories-heading" {
            div class="related-section-heading" {
                div {
                    h2 id="related-stories-heading" { "Related Stories" }
                    p { "Nearby stories, listed alphabetically for easier scanning." }
                }
                span class="count-badge" { (related.len()) }
            }
            ul class="related-list related-list-grid" {
                @for story in related {
                    (related_story_card(base_path, story))
                }
            }
        }
    }
}

pub fn tag_sidebar(base_path: &str, tags: &[TagCount]) -> Markup {
    html! {
        div class="sidebar-card" {
            h3 class="sidebar-title" { "Popular Tags" }
            div class="tag-cloud" {
                @for t in tags {
                    a class="sidebar-tag-item" href=(format!("{}/tag/{}", base_path, urlencoding::encode(&t.name))) {
                        span class="tag-name" { "#" (t.name) }
                        span class="tag-count" { (t.count) }
                    }
                }
            }
        }
    }
}


pub fn author_story_row(base_path: &str, story: &StorySummary) -> Markup {
    let display_title = display_story_title(&story.title);
    html! {
        li class="author-story-row" {
            div class="author-story-main" {
                a class="author-story-title" href=(format!("{}/story/{}", base_path, story.id)) { (display_title) }
                div class="author-story-meta" {
                    @if let Some(date) = &story.date {
                        span { (date) }
                    }
                    @if let Some(readers) = story.readers {
                        span { (readers) " readers" }
                    }
                }
            }
            @if let Some(tags) = &story.tags {
                div class="author-story-tags" {
                    @for tag in tags.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).take(4) {
                        a class="tag-pill" href=(format!("{}/tag/{}", base_path, urlencoding::encode(tag))) { (tag) }
                    }
                }
            }
        }
    }
}

pub fn story_card(base_path: &str, story: &StorySummary) -> Markup {
    let author_encoded = urlencoding::encode(&story.author);
    let display_title = display_story_title(&story.title);
    html! {
        article class="story-card" {
            h3 class="story-title" {
                a href=(format!("{}/story/{}", base_path, story.id)) { (display_title) }
            }
            p class="story-author" {
                "by "
                a href=(format!("{}/author/{}/", base_path, author_encoded)) { (story.author) }
            }
            div class="story-meta" {
                @if let Some(score) = story.score {
                    span class="badge badge-score" { "Relevance " (format!("{:.0}", score * 100.0)) }
                }
                @if let Some(readers) = story.readers {
                    span class="meta-item" { (readers) " readers" }
                }
                @if let Some(date) = &story.date {
                    span class="meta-item" { (date) }
                }
            }
            @if let Some(tags) = &story.tags {
                div class="pills-container" {
                    @for tag in tags.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                        a class="tag-pill" href=(format!("{}/tag/{}", base_path,urlencoding::encode(tag))) { (tag) }
                    }
                }
            }
            @if let Some(keywords) = &story.keywords {
                div class="pills-container" {
                    @for kw in keywords.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                        a class="kw-pill" href=(format!("{}/keyword/{}", base_path,urlencoding::encode(kw))) { (kw) }
                    }
                }
            }
            @if let Some(intro) = &story.introduction {
                @if !intro.trim().is_empty() {
                    p class="story-intro" { (intro) }
                }
            }
        }
    }
}

pub fn pagination_nav(base_path: &str, query_params: &str, pag: &Pagination) -> Markup {
    let prefix = if query_params.is_empty() {
        format!("{}?", base_path)
    } else {
        format!("{}?{}&", base_path, query_params)
    };

    html! {
        nav class="pagination" aria-label="Pagination" {
            @if pag.has_previous {
                a class="pagination-link" href=(format!("{}page=1", prefix)) { "« First" }
                a class="pagination-link" href=(format!("{}page={}", prefix, pag.current_page - 1)) { "‹ Prev" }
            }
            span class="pagination-current" {
                "Page " (pag.current_page) " of " (pag.total_pages.max(1))
            }
            @if pag.has_next {
                a class="pagination-link" href=(format!("{}page={}", prefix, pag.current_page + 1)) { "Next ›" }
                a class="pagination-link" href=(format!("{}page={}", prefix, pag.total_pages)) { "Last »" }
            }
        }
    }
}
