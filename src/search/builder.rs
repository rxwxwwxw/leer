// src/search/builder.rs
use rusqlite::types::ToSql;
use std::collections::HashSet;

pub fn parse_csv(input: Option<&str>) -> Vec<String> {
    let mut seen = HashSet::new();
    input
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .filter(|s| seen.insert(s.clone()))
        .collect()
}

fn push_string_params(values: &[String], params: &mut Vec<Box<dyn ToSql>>) -> String {
    let mut placeholders = Vec::with_capacity(values.len());
    for value in values {
        params.push(Box::new(value.clone()));
        placeholders.push(format!("?{}", params.len()));
    }
    placeholders.join(",")
}

pub fn build_where_filters(
    and_tags: &[String],
    or_tags: &[String],
    exc_tags: &[String],
    inc_kws: &[String],
    exc_kws: &[String],
    params: &mut Vec<Box<dyn ToSql>>,
) -> String {
    let mut filters = Vec::new();

    for tag in and_tags {
        let placeholder = push_string_params(std::slice::from_ref(tag), params);
        filters.push(format!("m.id IN (SELECT st.story_id FROM tags t JOIN story_tags st ON st.tag_id = t.id WHERE t.name COLLATE NOCASE = {})", placeholder));
    }

    if !or_tags.is_empty() {
        let placeholders = push_string_params(or_tags, params);
        filters.push(format!(
            "m.id IN (
                SELECT st.story_id FROM story_tags st
                JOIN tags t ON st.tag_id = t.id
                WHERE t.name COLLATE NOCASE IN ({})
            )",
            placeholders
        ));
    }

    if !exc_tags.is_empty() {
        let placeholders = push_string_params(exc_tags, params);
        filters.push(format!(
            "m.id NOT IN (
                SELECT st.story_id FROM story_tags st
                JOIN tags t ON st.tag_id = t.id
                WHERE t.name COLLATE NOCASE IN ({})
            )",
            placeholders
        ));
    }

    for keyword in inc_kws {
        let placeholder = push_string_params(std::slice::from_ref(keyword), params);
        filters.push(format!("m.id IN (SELECT sk.story_id FROM keywords k JOIN story_keywords sk ON sk.keyword_id = k.id WHERE k.name COLLATE NOCASE = {})", placeholder));
    }

    if !exc_kws.is_empty() {
        let placeholders = push_string_params(exc_kws, params);
        filters.push(format!(
            "m.id NOT IN (
                SELECT sk.story_id FROM story_keywords sk
                JOIN keywords k ON sk.keyword_id = k.id
                WHERE k.name COLLATE NOCASE IN ({})
            )",
            placeholders
        ));
    }

    if filters.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", filters.join(" AND "))
    }
}
