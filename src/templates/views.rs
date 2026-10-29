use super::components::{author_story_row, pagination_nav, related_stories_section, story_card, tag_sidebar};
use super::layout::base_layout;
use crate::models::list::{ReadingList, ReadingListItem};
use crate::models::search::{Pagination, SearchPageQuery, SearchResultPayload, SearchScope, SearchSort};
use crate::models::story::{
    display_story_title, render_story_body, AuthorDirectoryEntry, AuthorTagCoverage, AuthorsPageQuery,
    StoryDetail, StorySummary, TagCount,
};
use crate::utils::url;
use maud::{html, Markup, PreEscaped};

pub fn reading_lists_page_template(base_path: &str, lists: &[ReadingList], tags: &[TagCount]) -> Markup {
    let section_header = html! {
        div class="page-title-block" {
            h2 { "Your Reading Lists" }
            p class="subtitle" { "Manage your curated story collections." }
        }
    };
    let content = html! {
        div class="create-list-box" {
            h3 { "Create New List" }
            form class="create-list-form" action=(url(base_path, "/lists")) method="post" {
                input type="text" name="title" placeholder="List title (e.g. Sci-Fi Favorites)" required;
                label {
                    input type="checkbox" name="is_public" value="true";
                    "Public"
                }
                button type="submit" class="search-button" { "Create" }
            }
        }
        div class="lists-container" {
            @if lists.is_empty() {
                p class="no-results" { "You don't have any reading lists yet." }
            } @else {
                ul class="user-lists-grid" {
                    @for list in lists {
                        li class="user-list-item" {
                            div {
                                a class="user-list-title" href=(url(base_path, &format!("/lists/{}", list.id))) {
                                    (list.title)
                                }
                                div class="user-list-meta" {
                                    (list.story_count) " stories • " (if list.is_public { "Public" } else { "Private" })
                                }
                            }
                            form action=(url(base_path, &format!("/lists/{}/delete", list.id))) method="post" onsubmit="return confirm('Delete this list?');" {
                                button type="submit" class="btn-delete-list" { "Delete" }
                            }
                        }
                    }
                }
            }
        }
    };
    base_layout(base_path, "Reading Lists", None, None, section_header, content, Some(tag_sidebar(base_path, tags)), None, None)
}

pub fn reading_list_detail_template(
    base_path: &str,
    list: &ReadingList,
    items: &[ReadingListItem],
    tags: &[TagCount],
) -> Markup {
    let section_header = html! {
        div class="page-title-block" {
            h2 { (list.title) }
            span class="count-badge" { (items.len()) " stories" }
        }
    };
    let content = html! {
        div class="story-list" {
            @if items.is_empty() {
                p class="no-results" { "This list is currently empty." }
            } @else {
                @for item in items {
                    div class="list-item-wrapper" {
                        (story_card(base_path, &item.story))
                        @if let Some(notes) = &item.notes {
                            div class="list-item-note" {
                                strong { "Note: " } (notes)
                            }
                        }
                        form class="remove-item-form" action=(url(base_path, &format!("/lists/{}/items/{}/delete", list.id, item.story.id))) method="post" {
                            button type="submit" class="btn-remove-item" {
                                "Remove from list"
                            }
                        }
                    }
                }
            }
        }
    };
    base_layout(base_path, &list.title, None, None, section_header, content, Some(tag_sidebar(base_path, tags)), None, None)
}

fn search_form_component(base_path: &str, q: &SearchPageQuery, tags: &[TagCount]) -> Markup {
    let query_val = q.query.as_deref().unwrap_or("");
    let and_tags_val = q.and_tags.as_deref().unwrap_or("");
    let or_tags_val = q.or_tags.as_deref().unwrap_or("");
    let exc_tags_val = q.exc_tags.as_deref().unwrap_or("");
    let inc_kws_val = q.inc_kws.as_deref().unwrap_or("");
    let exc_kws_val = q.exc_kws.as_deref().unwrap_or("");
    let current_scope = q.scope.unwrap_or(SearchScope::All);
    let current_sort = q.sort.unwrap_or(SearchSort::Relevance);
    let has_filters = !and_tags_val.trim().is_empty()
        || !or_tags_val.trim().is_empty()
        || !exc_tags_val.trim().is_empty()
        || !inc_kws_val.trim().is_empty()
        || !exc_kws_val.trim().is_empty();
    let show_advanced = has_filters
        || current_scope != SearchScope::All
        || current_sort != SearchSort::Relevance;

    html! {
        form id="search-form" class="search-form-advanced" action=(url(base_path, "/search")) method="get" {
            input type="hidden" id="and_tags_input" name="and_tags" value=(and_tags_val);
            input type="hidden" id="or_tags_input" name="or_tags" value=(or_tags_val);
            input type="hidden" id="exc_tags_input" name="exc_tags" value=(exc_tags_val);

            div class="search-primary-row" {
                label class="search-query-field" {
                    span class="sr-only" { "Search the archive" }
                    input id="query" type="search" name="query" value=(query_val)
                        placeholder="Search titles, authors, themes, or story text…"
                        autocomplete="off" autofocus;
                }
                button class="search-button search-button-primary" type="submit" { "Search" }
            }

            div class="search-subline" {
                span { "Hybrid lexical + semantic search" }
                span class="search-subline-separator" { "•" }
                span { "Press " kbd { "/" } " to focus search" }
            }

            details class="search-advanced" open[show_advanced] {
                summary class="search-advanced-summary" {
                    span { "Search options & filters" }
                    @if has_filters {
                        span class="active-filter-badge" { "Filters active" }
                    }
                }
                div class="search-advanced-body" {
                    div class="search-controls-grid" {
                        label class="search-control" {
                            span class="search-field-label" { "Search in" }
                            select name="scope" id="scope-select" class="search-select" onchange="toggleScopeControls(this.value)" {
                                option value="all" selected[current_scope == SearchScope::All] { "Everything" }
                                option value="title" selected[current_scope == SearchScope::Title] { "Titles" }
                                option value="author" selected[current_scope == SearchScope::Author] { "Authors" }
                                option value="content" selected[current_scope == SearchScope::Content] { "Story text" }
                            }
                        }
                        label id="sort-wrapper" class="search-control" style=(if current_scope == SearchScope::Author { "display:none" } else { "" }) {
                            span class="search-field-label" { "Sort" }
                            select name="sort" id="sort-select" class="search-select" {
                                option value="relevance" selected[current_sort == SearchSort::Relevance] { "Best match" }
                                option value="newest" selected[current_sort == SearchSort::Newest] { "Newest first" }
                                option value="oldest" selected[current_sort == SearchSort::Oldest] { "Oldest first" }
                                option value="title" selected[current_sort == SearchSort::Title] { "Title A–Z" }
                            }
                        }
                    }

                    div id="story-filter-controls" style=(if current_scope == SearchScope::Author { "display:none" } else { "" }) {
                        section class="filter-section" {
                            div class="tag-filter-header" {
                                div {
                                    span class="search-field-label" { "Tags" }
                                    p class="filter-help" { "Click a tag repeatedly to require it, prefer any of a group, exclude it, or return to neutral." }
                                }
                                button type="button" class="btn-reset-filters" onclick="clearAllTags()" { "Clear tags" }
                            }
                            div class="filter-toolbar" {
                                input id="tag-filter-search" class="filter-search-input" type="search" placeholder="Find a tag…" oninput="filterTagButtons(this.value)";
                                div class="tag-state-legend" aria-label="Tag filter states" {
                                    span class="legend-chip legend-and" { "+ Required" }
                                    span class="legend-chip legend-or" { "? Any" }
                                    span class="legend-chip legend-exclude" { "− Excluded" }
                                }
                            }
                            div class="tri-tag-grid" id="tag-grid" {
                                @for t in tags {
                                    button type="button" class="tri-tag-btn" data-tag=(t.name) data-state="neutral" onclick="cycleTag(this)" {
                                        span class="tri-tag-name" { (t.name) }
                                        span class="tri-tag-count" { (t.count) }
                                    }
                                }
                            }
                        }

                        section class="filter-section keyword-filter-section" {
                            div class="tag-filter-header" {
                                div {
                                    span class="search-field-label" { "Keywords" }
                                    p class="filter-help" { "Comma-separated exact keyword filters." }
                                }
                            }
                            div class="kw-inputs-flex" {
                                label class="keyword-input-group" {
                                    span { "Must include all" }
                                    input id="inc-kws-input" type="text" name="inc_kws" value=(inc_kws_val) placeholder="magic, space";
                                }
                                label class="keyword-input-group" {
                                    span { "Exclude any" }
                                    input id="exc-kws-input" type="text" name="exc_kws" value=(exc_kws_val) placeholder="angst, gore";
                                }
                            }
                        }
                    }

                    div class="search-filter-footer" {
                        div id="active-filter-summary" class="active-filter-summary" aria-live="polite" {}
                        button type="button" class="btn-clear-all" onclick="clearAllSearchOptions()" { "Reset all options" }
                    }
                }
            }
        }
        script {
            (PreEscaped(r#"
            const TAG_STATE_LABELS = {
                neutral: '',
                and: '+',
                or: '?',
                exclude: '−'
            };

            function setTagState(btn, state) {
                btn.setAttribute('data-state', state);
                const tag = btn.getAttribute('data-tag');
                const name = btn.querySelector('.tri-tag-name');
                if (name) name.textContent = (TAG_STATE_LABELS[state] ? TAG_STATE_LABELS[state] + ' ' : '') + tag;
                btn.setAttribute('aria-label', `${tag}: ${state === 'neutral' ? 'not filtered' : state}`);
            }

            function cycleTag(btn) {
                const states = ['neutral', 'and', 'or', 'exclude'];
                const current = btn.getAttribute('data-state') || 'neutral';
                setTagState(btn, states[(states.indexOf(current) + 1) % states.length]);
                syncTagInputs();
            }

            function syncTagInputs() {
                const buckets = { and: [], or: [], exclude: [] };
                document.querySelectorAll('.tri-tag-btn').forEach(btn => {
                    const state = btn.getAttribute('data-state');
                    if (buckets[state]) buckets[state].push(btn.getAttribute('data-tag'));
                });
                document.getElementById('and_tags_input').value = buckets.and.join(',');
                document.getElementById('or_tags_input').value = buckets.or.join(',');
                document.getElementById('exc_tags_input').value = buckets.exclude.join(',');
                updateActiveFilterSummary();
            }

            function clearAllTags() {
                document.querySelectorAll('.tri-tag-btn').forEach(btn => setTagState(btn, 'neutral'));
                syncTagInputs();
            }

            function filterTagButtons(value) {
                const needle = value.trim().toLowerCase();
                document.querySelectorAll('.tri-tag-btn').forEach(btn => {
                    const matches = !needle || btn.getAttribute('data-tag').toLowerCase().includes(needle);
                    btn.hidden = !matches;
                });
            }

            function toggleScopeControls(value) {
                const authorOnly = value === 'author';
                const storyControls = document.getElementById('story-filter-controls');
                const sortWrapper = document.getElementById('sort-wrapper');
                if (storyControls) storyControls.style.display = authorOnly ? 'none' : '';
                if (sortWrapper) sortWrapper.style.display = authorOnly ? 'none' : '';

                const query = document.getElementById('query');
                if (query) {
                    query.placeholder = authorOnly
                        ? 'Search author names…'
                        : value === 'title'
                            ? 'Search story titles…'
                            : value === 'content'
                                ? 'Search story text and themes…'
                                : 'Search titles, authors, themes, or story text…';
                }
                updateActiveFilterSummary();
            }

            function csvCount(id) {
                const el = document.getElementById(id);
                if (!el || !el.value.trim()) return 0;
                return el.value.split(',').map(s => s.trim()).filter(Boolean).length;
            }

            function updateActiveFilterSummary() {
                const summary = document.getElementById('active-filter-summary');
                if (!summary) return;
                const scope = document.getElementById('scope-select')?.value || 'all';
                const sort = document.getElementById('sort-select')?.value || 'relevance';
                const required = csvCount('and_tags_input');
                const any = csvCount('or_tags_input');
                const excluded = csvCount('exc_tags_input');
                const incKw = csvCount('inc-kws-input');
                const excKw = csvCount('exc-kws-input');
                const parts = [];
                if (scope !== 'all') parts.push(`scope: ${scope}`);
                if (sort !== 'relevance' && scope !== 'author') parts.push(`sort: ${sort}`);
                if (required) parts.push(`${required} required tag${required === 1 ? '' : 's'}`);
                if (any) parts.push(`${any} any-tag${any === 1 ? '' : 's'}`);
                if (excluded) parts.push(`${excluded} excluded tag${excluded === 1 ? '' : 's'}`);
                if (incKw) parts.push(`${incKw} required keyword${incKw === 1 ? '' : 's'}`);
                if (excKw) parts.push(`${excKw} excluded keyword${excKw === 1 ? '' : 's'}`);
                summary.textContent = parts.length ? parts.join(' · ') : 'No filters applied';
            }

            function clearAllSearchOptions() {
                clearAllTags();
                const inc = document.getElementById('inc-kws-input');
                const exc = document.getElementById('exc-kws-input');
                const scope = document.getElementById('scope-select');
                const sort = document.getElementById('sort-select');
                const finder = document.getElementById('tag-filter-search');
                if (inc) inc.value = '';
                if (exc) exc.value = '';
                if (scope) scope.value = 'all';
                if (sort) sort.value = 'relevance';
                if (finder) { finder.value = ''; filterTagButtons(''); }
                toggleScopeControls('all');
                updateActiveFilterSummary();
            }

            function initTagsFromUrl() {
                const selected = {
                    and: (document.getElementById('and_tags_input').value || '').split(',').map(s => s.trim().toLowerCase()).filter(Boolean),
                    or: (document.getElementById('or_tags_input').value || '').split(',').map(s => s.trim().toLowerCase()).filter(Boolean),
                    exclude: (document.getElementById('exc_tags_input').value || '').split(',').map(s => s.trim().toLowerCase()).filter(Boolean)
                };
                document.querySelectorAll('.tri-tag-btn').forEach(btn => {
                    const tag = btn.getAttribute('data-tag').toLowerCase();
                    const state = selected.and.includes(tag) ? 'and' : selected.or.includes(tag) ? 'or' : selected.exclude.includes(tag) ? 'exclude' : 'neutral';
                    setTagState(btn, state);
                });
                toggleScopeControls(document.getElementById('scope-select')?.value || 'all');
                updateActiveFilterSummary();
            }

            document.addEventListener('DOMContentLoaded', () => {
                initTagsFromUrl();
                ['inc-kws-input', 'exc-kws-input', 'sort-select'].forEach(id => {
                    document.getElementById(id)?.addEventListener('input', updateActiveFilterSummary);
                    document.getElementById(id)?.addEventListener('change', updateActiveFilterSummary);
                });
            });

            document.addEventListener('keydown', event => {
                if (event.key === '/' && !event.ctrlKey && !event.metaKey && !event.altKey) {
                    const target = event.target;
                    const typing = target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT' || target.isContentEditable);
                    if (!typing) {
                        event.preventDefault();
                        document.getElementById('query')?.focus();
                    }
                }
            });
            "#))
        }
    }
}

pub fn search_query_template(base_path: &str, tags: &[TagCount], query: &SearchPageQuery) -> Markup {
    let section_header = html! {
        div class="page-title-block" {
            h2 { "Story Archive Search" }
            p class="subtitle" { "Fast hybrid search across titles, authors, story text, tags, and semantic similarity." }
        }
    };
    let content = html! {
        div class="search-box-container" {
            (search_form_component(base_path, query, tags))
        }
    };
    base_layout(base_path, "Search", None, None, section_header, content, Some(tag_sidebar(base_path, tags)), None, None)
}

pub fn search_results_template(
    base_path: &str,
    query: &SearchPageQuery,
    payload: &SearchResultPayload,
    total_count: usize,
    pagination: &Pagination,
    tags: &[TagCount],
) -> Markup {
    let title = format!("Search: {}", query.query.as_deref().unwrap_or("Filtered Search"));
    let query_text = query.query.as_deref().unwrap_or("").trim();
    let result_label = match payload {
        SearchResultPayload::Authors(_) => if total_count == 1 { "author" } else { "authors" },
        SearchResultPayload::Stories(_) => if total_count == 1 { "story" } else { "stories" },
    };
    let section_header = html! {
        div class="search-box-container" {
            (search_form_component(base_path, query, tags))
        }
    };
    let content = match payload {
        SearchResultPayload::Authors(authors) => html! {
            div class="results-heading" {
                div {
                    h2 { "Authors" }
                    p class="results-summary" {
                        (total_count) " " (result_label)
                        @if !query_text.is_empty() { " matching “" (query_text) "”" }
                    }
                }
            }
            div class="author-results-container" {
                @if authors.is_empty() {
                    div class="empty-state" {
                        h3 { "No matching authors" }
                        p { "Try a shorter spelling or switch the search scope back to Everything." }
                    }
                } @else {
                    ul class="author-results-list" {
                        @for author in authors {
                            li class="author-result-item" {
                                a class="author-result-link" href=(url(base_path, &format!("/author/{}/", urlencoding::encode(&author.name)))) {
                                    (author.name)
                                }
                                span class="badge" { (author.story_count) " stories" }
                            }
                        }
                    }
                }
            }
        },
        SearchResultPayload::Stories(results) => html! {
            div class="results-heading" {
                div {
                    h2 { "Stories" }
                    p class="results-summary" {
                        (total_count) " " (result_label)
                        @if !query_text.is_empty() { " matching “" (query_text) "”" }
                    }
                }
                @if pagination.total_pages > 1 {
                    span class="page-position" { "Page " (pagination.current_page) " of " (pagination.total_pages) }
                }
            }
            div class="story-results-list story-list" {
                @if results.is_empty() {
                    div class="empty-state" {
                        h3 { "No stories found" }
                        p { "Try broadening the query, removing a required filter, or searching Everything." }
                    }
                } @else {
                    @for row in results {
                        (story_card(base_path, row))
                    }
                }
            }
        },
    };
    let mut qs = Vec::new();
    if let Some(q) = &query.query { qs.push(format!("query={}", urlencoding::encode(q))); }
    if let Some(s) = &query.scope { qs.push(format!("scope={:?}", s).to_lowercase()); }
    if let Some(s) = &query.sort { qs.push(format!("sort={:?}", s).to_lowercase()); }
    if let Some(t) = &query.and_tags { qs.push(format!("and_tags={}", urlencoding::encode(t))); }
    if let Some(t) = &query.or_tags { qs.push(format!("or_tags={}", urlencoding::encode(t))); }
    if let Some(t) = &query.exc_tags { qs.push(format!("exc_tags={}", urlencoding::encode(t))); }
    if let Some(k) = &query.inc_kws { qs.push(format!("inc_kws={}", urlencoding::encode(k))); }
    if let Some(k) = &query.exc_kws { qs.push(format!("exc_kws={}", urlencoding::encode(k))); }

    let footer = if total_count > 0 {
        Some(html! { (pagination_nav(&url(base_path, "/search"), &qs.join("&"), pagination)) })
    } else {
        None
    };
    base_layout(base_path, &title, None, None, section_header, content, Some(tag_sidebar(base_path, tags)), None, footer)
}

pub fn home_page_template(
    base_path: &str,
    featured_kind: &str,
    featured_name: &str,
    featured_stories: &[StorySummary],
    random_stories: &[StorySummary],
    tags: &[TagCount],
) -> Markup {
    let title = "Home";
    let target_url = if featured_kind == "Tag" {
        url(base_path, &format!("/tag/{}", urlencoding::encode(featured_name)))
    } else {
        url(base_path, &format!("/keyword/{}", urlencoding::encode(featured_name)))
    };
    let section_header = html! {
        div class="home-hero" {
            h1 { "Discover Stories" }
            p class="subtitle" { "Explore curated picks and random selections from the archive." }
        }
    };
    let content = html! {
        section class="home-section" {
            div class="section-title-row" {
                h2 {
                    "Featured " (featured_kind) ": "
                    a class="featured-highlight" href=(target_url) { "#" (featured_name) }
                }
                span class="badge" { "50 Random Picks" }
            }
            div class="story-list" {
                @for story in featured_stories {
                    (story_card(base_path, story))
                }
            }
        }
        hr class="section-divider";
        section class="home-section" {
            div class="section-title-row" {
                h2 { "More Random Discoveries" }
                span class="badge" { "50 Stories" }
            }
            div class="story-list" {
                @for story in random_stories {
                    (story_card(base_path, story))
                }
            }
        }
    };
    base_layout(base_path, title, None, None, section_header, content, Some(tag_sidebar(base_path, tags)), None, None)
}

pub fn category_index_template(
    base_path: &str,
    kind: &str,
    name: &str,
    total_in_pool: usize,
    stories: &[StorySummary],
    pagination: &Pagination,
    seed: i64,
    tags: &[TagCount],
) -> Markup {
    let title = format!("{}: {}", kind, name);
    let section_header = html! {
        div class="page-title-block" {
            h2 { (kind) ": #" (name) }
            span class="count-badge" { (total_in_pool) " stories sampled" }
        }
    };
    let content = html! {
        div class="story-list" {
            @if stories.is_empty() {
                p class="no-results" { "No stories found." }
            } @else {
                @for story in stories {
                    (story_card(base_path, story))
                }
            }
        }
    };
    let base_route = if kind == "Tag" {
        url(base_path, &format!("/tag/{}", urlencoding::encode(name)))
    } else {
        url(base_path, &format!("/keyword/{}", urlencoding::encode(name)))
    };
    let query_param = format!("seed={}", seed);
    let footer = html! {
        (pagination_nav(&base_route, &query_param, pagination))
    };
    base_layout(base_path, &title, None, None, section_header, content, Some(tag_sidebar(base_path, tags)), None, Some(footer))
}

pub fn story_detail_template(
    base_path: &str,
    story: &StoryDetail,
    related: &[StorySummary],
    is_raw: bool,
) -> Markup {
    let display_title = display_story_title(&story.title);
    let title = format!("{} by {}", display_title, story.author);
    let author_encoded = urlencoding::encode(&story.author);
    let toggle_url = if is_raw {
        url(base_path, &format!("/story/{}", story.id))
    } else {
        url(base_path, &format!("/story/{}?raw=true", story.id))
    };
    let toggle_label = if is_raw {
        "View Formatted (Markdown)"
    } else {
        "View Plain Text (Raw)"
    };
    let section_header = html! {
        header class="story-header" {
            div class="story-header-layout" {
                div class="story-heading-copy" {
                    h1 class="story-title-main" { (display_title) }
                    p class="story-author-byline" {
                        "Written by "
                        a href=(url(base_path, &format!("/author/{}/", author_encoded))) { (story.author) }
                        " • Published " (story.datetime)
                    }
                }
                a href=(toggle_url) class="toggle-raw-btn" {
                    (toggle_label)
                }
            }
            div class="story-meta-stats" {
                span { (story.upvotes) " Upvotes" }
                span { (story.downvotes) " Downvotes" }
                span { (story.readers) " Readers" }
            }
            @if let Some(intro) = &story.introduction {
                @if !intro.trim().is_empty() {
                    div class="story-introduction" {
                        p { (intro) }
                    }
                }
            }
            @if story.tags.as_ref().is_some_and(|v| !v.trim().is_empty()) || story.keywords.as_ref().is_some_and(|v| !v.trim().is_empty()) {
                div class="story-taxonomy" {
                    @if let Some(tags) = &story.tags {
                        @if !tags.trim().is_empty() {
                            div class="story-taxonomy-row" {
                                span class="story-taxonomy-label" { "Tags" }
                                div class="pills-container story-header-pills" {
                                    @for tag in tags.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                                        a class="tag-pill" href=(url(base_path, &format!("/tag/{}", urlencoding::encode(tag)))) { (tag) }
                                    }
                                }
                            }
                        }
                    }
                    @if let Some(keywords) = &story.keywords {
                        @if !keywords.trim().is_empty() {
                            div class="story-taxonomy-row" {
                                span class="story-taxonomy-label" { "Keywords" }
                                div class="pills-container story-header-pills" {
                                    @for keyword in keywords.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                                        a class="kw-pill" href=(url(base_path, &format!("/keyword/{}", urlencoding::encode(keyword)))) { (keyword) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };
    let content = html! {
        article class="story-reader" {
            @if is_raw {
                pre class="story-raw-content" {
                    (story.content)
                }
            } @else {
                div class="story-markdown-content" {
                    (render_story_body(&story.content))
                }
            }
        }
    };
    let related_section = if related.is_empty() {
        None
    } else {
        Some(related_stories_section(base_path, related))
    };
    base_layout(
        base_path,
        &title,
        None,
        None,
        section_header,
        content,
        None,
        related_section,
        None,
    )
}

pub fn author_index_template(
    base_path: &str,
    author: &str,
    stories: &[StorySummary],
    total_count: usize,
    pagination: &Pagination,
) -> Markup {
    let title = format!("Author: {}", author);
    let section_header = html! {
        div class="page-title-block author-page-heading" {
            div {
                h2 { "Stories by " (author) }
                p class="subtitle" { "Alphabetical by title." }
            }
            span class="count-badge" { (total_count) " works" }
        }
    };
    let content = html! {
        @if stories.is_empty() {
            div class="empty-state" {
                h3 { "No stories found" }
                p { "This author does not currently have any stories in the archive." }
            }
        } @else {
            ul class="author-story-list" {
                @for row in stories {
                    (author_story_row(base_path, row))
                }
            }
        }
    };
    let footer = if pagination.total_pages > 1 {
        Some(html! {
            (pagination_nav(
                &url(base_path, &format!("/author/{}/", urlencoding::encode(author))),
                "",
                pagination,
            ))
        })
    } else {
        None
    };
    base_layout(base_path, &title, None, None, section_header, content, None, None, footer)
}

pub fn authors_index_template(
    base_path: &str,
    query: &AuthorsPageQuery,
    authors: &[AuthorDirectoryEntry],
    total_count: usize,
    pagination: &Pagination,
) -> Markup {
    let query_text = query.query.as_deref().unwrap_or("");
    let coverage = query.tag_coverage.unwrap_or_default();
    let min_stories = query.min_stories.unwrap_or(1).max(1);
    let section_header = html! {
        div class="page-title-block" {
            h1 { "Authors" }
            p class="subtitle" { "Browse the archive alphabetically and filter authors by story/tag coverage." }
        }
    };
    let content = html! {
        form class="author-directory-filters" action=(url(base_path, "/authors")) method="get" {
            label class="author-filter-field author-filter-search" {
                span { "Author name" }
                input type="search" name="query" value=(query_text) placeholder="Filter authors…";
            }
            label class="author-filter-field" {
                span { "Tag coverage" }
                select name="tag_coverage" {
                    option value="all" selected[coverage == AuthorTagCoverage::All] { "All authors" }
                    option value="any" selected[coverage == AuthorTagCoverage::Any] { "At least one tagged story" }
                    option value="none" selected[coverage == AuthorTagCoverage::None] { "No tagged stories" }
                    option value="all_tagged" selected[coverage == AuthorTagCoverage::AllTagged] { "Every story tagged" }
                    option value="mixed" selected[coverage == AuthorTagCoverage::Mixed] { "Mixed tagged / untagged" }
                }
            }
            label class="author-filter-field author-filter-min" {
                span { "Minimum stories" }
                input type="number" name="min_stories" min="1" value=(min_stories);
            }
            div class="author-filter-actions" {
                button class="search-button" type="submit" { "Apply" }
                a class="secondary-link-button" href=(url(base_path, "/authors")) { "Reset" }
            }
        }

        div class="results-heading author-directory-heading" {
            div {
                h2 { "Author directory" }
                p class="results-summary" { (total_count) " authors" }
            }
            @if pagination.total_pages > 1 {
                span class="page-position" { "Page " (pagination.current_page) " of " (pagination.total_pages) }
            }
        }

        @if authors.is_empty() {
            div class="empty-state" {
                h3 { "No matching authors" }
                p { "Try changing the name, tag-coverage filter, or minimum story count." }
            }
        } @else {
            div class="author-directory-list" {
                @for author in authors {
                    a class="author-directory-row" href=(url(base_path, &format!("/author/{}/", urlencoding::encode(&author.name)))) {
                        span class="author-directory-name" { (author.name) }
                        span class="author-directory-counts" {
                            strong { (author.story_count) }
                            " stories · "
                            (author.tagged_story_count)
                            " tagged · "
                            (author.story_count - author.tagged_story_count)
                            " untagged"
                        }
                    }
                }
            }
        }
    };

    let mut qs = Vec::new();
    if !query_text.trim().is_empty() {
        qs.push(format!("query={}", urlencoding::encode(query_text.trim())));
    }
    if coverage != AuthorTagCoverage::All {
        let coverage_param = match coverage {
            AuthorTagCoverage::All => "all",
            AuthorTagCoverage::Any => "any",
            AuthorTagCoverage::None => "none",
            AuthorTagCoverage::AllTagged => "all_tagged",
            AuthorTagCoverage::Mixed => "mixed",
        };
        qs.push(format!("tag_coverage={}", coverage_param));
    }
    if min_stories > 1 {
        qs.push(format!("min_stories={}", min_stories));
    }
    let footer = if pagination.total_pages > 1 {
        Some(html! {
            (pagination_nav(&url(base_path, "/authors"), &qs.join("&"), pagination))
        })
    } else {
        None
    };

    base_layout(
        base_path,
        "Authors",
        None,
        None,
        section_header,
        content,
        None,
        None,
        footer,
    )
}

pub fn login_page_template(base_path: &str, error: Option<&str>, tags: &[TagCount]) -> Markup {
    let section_header = html! {
        div class="page-title-block" {
            h2 { "Sign In" }
            p class="subtitle" { "Access your reading lists and saved collections." }
        }
    };
    let content = html! {
        div class="auth-card-container" {
            @if let Some(err) = error {
                div class="auth-error-alert" {
                    (err)
                }
            }
            form action=(url(base_path, "/login")) method="post" class="auth-form-layout" {
                label class="auth-input-group" {
                    "Username"
                    input type="text" name="username" required autofocus;
                }
                label class="auth-input-group" {
                    "Password"
                    input type="password" name="password" required;
                }
                button type="submit" class="search-button auth-btn-submit" { "Sign In" }
            }
            p class="auth-footer-text" {
                "Don't have an account? " a href=(url(base_path, "/register")) { "Register here" }
            }
        }
    };
    base_layout(base_path, "Login", None, None, section_header, content, Some(tag_sidebar(base_path, tags)), None, None)
}

pub fn register_page_template(base_path: &str, error: Option<&str>, tags: &[TagCount]) -> Markup {
    let section_header = html! {
        div class="page-title-block" {
            h2 { "Create Account" }
            p class="subtitle" { "Register to create personalized story reading lists." }
        }
    };
    let content = html! {
        div class="auth-card-container" {
            @if let Some(err) = error {
                div class="auth-error-alert" {
                    (err)
                }
            }
            form action=(url(base_path, "/register")) method="post" class="auth-form-layout" {
                label class="auth-input-group" {
                    "Username"
                    input type="text" name="username" required autofocus minlength="3" maxlength="32";
                }
                label class="auth-input-group" {
                    "Password"
                    input type="password" name="password" required minlength="8";
                }
                label class="auth-input-group" {
                    "Confirm Password"
                    input type="password" name="confirm_password" required minlength="8";
                }
                button type="submit" class="search-button auth-btn-submit" { "Register" }
            }
            p class="auth-footer-text" {
                "Already have an account? " a href=(url(base_path, "/login")) { "Sign in here" }
            }
        }
    };
    base_layout(base_path, "Register", None, None, section_header, content, Some(tag_sidebar(base_path, tags)), None, None)
}
