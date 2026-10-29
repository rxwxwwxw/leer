// src/search/engine.rs
use crate::app_state::AppState;
use crate::models::search::{
    AuthorSearchResult, SearchPageQuery, SearchResultPayload, SearchScope, SearchSort,
};
use crate::models::story::StorySummary;
use crate::search::builder::{build_where_filters, parse_csv};
use anyhow::{anyhow, Result};
use rusqlite::types::ToSql;
use std::collections::HashSet;
use zerocopy::IntoBytes;

const PORTER_WEIGHT: f32 = 1.0;
const TRIGRAM_WEIGHT: f32 = 0.45;
const VECTOR_WEIGHT: f32 = 0.75;
const PROXIMITY_WEIGHT: f32 = 1.20;
const PHRASE_WEIGHT: f32 = 1.40;
const EXACT_TITLE_BONUS: f32 = 0.03;

/// Turn user text into a conservative FTS5 query where every whitespace-delimited
/// token is required. Quoting each token prevents FTS operators in user input from
/// changing the query grammar.
pub fn sanitize_fts(q: &str) -> String {
    q.split_whitespace()
        .map(|t| t.replace('"', ""))
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{}\"", t))
        .collect::<Vec<_>>()
        .join(" ")
}

fn sanitize_phrase(q: &str) -> String {
    let clean = q
        .split_whitespace()
        .map(|t| t.replace('"', ""))
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    format!("\"{}\"", clean)
}

fn apply_scope(query: &str, scope: SearchScope) -> String {
    match scope {
        SearchScope::Title => format!("title: ({})", query),
        SearchScope::Content => format!("content: ({})", query),
        SearchScope::All | SearchScope::Author => query.to_string(),
    }
}

fn build_trigram_query(q: &str, scope: SearchScope) -> Option<String> {
    let mut seen = HashSet::new();
    let mut trigrams = Vec::new();

    for word in q.split_whitespace() {
        let clean = word
            .chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(|c| c.to_lowercase())
            .collect::<Vec<_>>();
        if clean.len() < 3 {
            continue;
        }
        for window in clean.windows(3) {
            let tri = window.iter().collect::<String>();
            if seen.insert(tri.clone()) {
                trigrams.push(tri);
                if trigrams.len() >= 24 {
                    break;
                }
            }
        }
        if trigrams.len() >= 24 {
            break;
        }
    }

    if trigrams.is_empty() {
        return None;
    }

    let expr = trigrams
        .into_iter()
        .map(|t| format!("\"{}\"", t))
        .collect::<Vec<_>>()
        .join(" OR ");

    Some(match scope {
        SearchScope::Title => format!("title: ({})", expr),
        SearchScope::Author => format!("author: ({})", expr),
        SearchScope::All => format!("{{title author}}: ({})", expr),
        SearchScope::Content => expr,
    })
}

pub fn build_proximity_boost_query(q: &str) -> Option<String> {
    let stop_words: HashSet<&'static str> = [
        "the", "a", "an", "and", "or", "but", "if", "then", "else", "of", "to", "in", "on",
        "at", "by", "for", "with", "is", "was", "are", "be", "from", "it", "as", "that", "this",
    ]
    .iter()
    .cloned()
    .collect();

    let terms: Vec<String> = q
        .split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|w| !w.is_empty() && !stop_words.contains(w.as_str()))
        .collect();

    if terms.len() < 2 {
        return None;
    }

    let clauses = terms
        .windows(2)
        .map(|pair| format!("NEAR(\"{}\" \"{}\", 10)", pair[0], pair[1]))
        .collect::<Vec<_>>();

    (!clauses.is_empty()).then(|| clauses.join(" OR "))
}

fn max_score_for_scope(scope: SearchScope, has_proximity: bool, has_trigram: bool) -> f32 {
    let mut score = PORTER_WEIGHT / 61.0 + PHRASE_WEIGHT / 41.0;
    if has_trigram {
        score += TRIGRAM_WEIGHT / 21.0;
    }
    if scope != SearchScope::Title {
        score += VECTOR_WEIGHT / 61.0;
    }
    if has_proximity {
        score += PROXIMITY_WEIGHT / 61.0;
    }
    if scope != SearchScope::Content {
        score += EXACT_TITLE_BONUS;
    }
    score
}

async fn execute_filter_only_search(
    conn: deadpool_sqlite::Object,
    query_params: &SearchPageQuery,
    sort_order: SearchSort,
    offset: usize,
    page_size: usize,
) -> Result<(SearchResultPayload, usize)> {
    let and_tags = parse_csv(query_params.and_tags.as_deref());
    let or_tags = parse_csv(query_params.or_tags.as_deref());
    let exc_tags = parse_csv(query_params.exc_tags.as_deref());
    let inc_kws = parse_csv(query_params.inc_kws.as_deref());
    let exc_kws = parse_csv(query_params.exc_kws.as_deref());

    let (results, total) = conn
        .interact(move |conn| -> Result<(Vec<StorySummary>, usize)> {
            let mut sql_params: Vec<Box<dyn ToSql>> = Vec::new();
            // Filters-only searches do not have ranker candidate pools. Apply the
            // requested tag/keyword filters directly to the metadata query.
            let where_clause = build_where_filters(
                &and_tags,
                &or_tags,
                &exc_tags,
                &inc_kws,
                &exc_kws,
                &mut sql_params,
            );

            sql_params.push(Box::new(page_size as i64));
            sql_params.push(Box::new(offset as i64));
            let limit_idx = sql_params.len() - 1;
            let offset_idx = sql_params.len();

            let order_by = match sort_order {
                SearchSort::Relevance => "ORDER BY COALESCE(m.readers, 0) DESC, m.timestamp DESC",
                SearchSort::Newest => "ORDER BY m.timestamp DESC",
                SearchSort::Oldest => "ORDER BY m.timestamp ASC",
                SearchSort::Title => "ORDER BY COALESCE(NULLIF(TRIM(m.title), ''), '(not title)') COLLATE NOCASE ASC",
            };

            let sql = format!(
                r#"
                SELECT m.id, m.title, a.name AS author, COALESCE(m.introduction, '') AS introduction,
                       m.readers,
                       strftime('%Y-%m-%d', datetime(m.timestamp, 'unixepoch')) AS date_str,
                       (SELECT GROUP_CONCAT(t.name, ', ') FROM story_tags st JOIN tags t ON st.tag_id = t.id WHERE st.story_id = m.id) AS tags,
                       (SELECT GROUP_CONCAT(k.name, ', ') FROM story_keywords sk JOIN keywords k ON sk.keyword_id = k.id WHERE sk.story_id = m.id) AS keywords,
                       COUNT(*) OVER() AS total_results
                FROM stories_metadata m
                JOIN authors a ON m.author_id = a.id
                {}
                {}
                LIMIT ?{} OFFSET ?{};
                "#,
                where_clause, order_by, limit_idx, offset_idx
            );

            let mut stmt = conn.prepare(&sql)?;
            let mut total_matches = 0;
            let rows = stmt.query_map(
                rusqlite::params_from_iter(sql_params.iter().map(|b| b.as_ref())),
                |r| {
                    total_matches = r.get::<_, i64>(8).unwrap_or(0) as usize;
                    Ok(StorySummary {
                        id: r.get(0)?,
                        title: r.get(1)?,
                        author: r.get(2)?,
                        introduction: r.get(3)?,
                        score: None,
                        readers: r.get(4)?,
                        date: r.get(5)?,
                        tags: r.get(6)?,
                        keywords: r.get(7)?,
                    })
                },
            )?;
            let list = rows.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok((list, total_matches))
        })
        .await
        .map_err(|e| anyhow!("DB error: {}", e))??;

    Ok((SearchResultPayload::Stories(results), total))
}

pub async fn execute_search(
    state: &AppState,
    query_params: &SearchPageQuery,
    page: usize,
    page_size: usize,
) -> Result<(SearchResultPayload, usize)> {
    let raw_q = query_params.query.as_deref().unwrap_or("").trim();
    let scope = query_params.scope.unwrap_or(SearchScope::All);
    let sort_order = query_params.sort.unwrap_or(SearchSort::Relevance);
    let offset = (page.saturating_sub(1)) * page_size;
    let conn = state.stories_pool.get().await?;

    if scope == SearchScope::Author {
        let author_query = raw_q.to_string();
        let author_trigram_query = build_trigram_query(raw_q, SearchScope::Author);
        let use_trigram = author_trigram_query.is_some();
        let (authors, total) = conn
            .interact(move |conn| -> Result<(Vec<AuthorSearchResult>, usize)> {
                let sql = if use_trigram {
                    r#"
                    WITH trigram_hits AS (
                        SELECT rowid AS story_id, rank AS match_rank
                        FROM trigram_fts
                        WHERE trigram_fts MATCH ?1
                        ORDER BY rank
                        LIMIT 3000
                    ),
                    matched_authors AS (
                        SELECT a.id, a.name, MIN(th.match_rank) AS match_rank
                        FROM trigram_hits th
                        JOIN stories_metadata m ON th.story_id = m.id
                        JOIN authors a ON m.author_id = a.id
                        GROUP BY a.id, a.name
                    ),
                    counted AS (
                        SELECT ma.id, ma.name, ma.match_rank, COUNT(m.id) AS story_count
                        FROM matched_authors ma
                        JOIN stories_metadata m ON ma.id = m.author_id
                        GROUP BY ma.id, ma.name, ma.match_rank
                    )
                    SELECT name, story_count, COUNT(*) OVER() AS total_count
                    FROM counted
                    ORDER BY CASE WHEN LOWER(name) = LOWER(?2) THEN 0 ELSE 1 END,
                             match_rank ASC, story_count DESC, name COLLATE NOCASE ASC
                    LIMIT ?3 OFFSET ?4;
                    "#
                } else {
                    r#"
                    WITH counted AS (
                        SELECT a.id, a.name, COUNT(m.id) AS story_count
                        FROM authors a
                        JOIN stories_metadata m ON a.id = m.author_id
                        WHERE LOWER(a.name) LIKE '%' || LOWER(?1) || '%'
                        GROUP BY a.id, a.name
                    )
                    SELECT name, story_count, COUNT(*) OVER() AS total_count
                    FROM counted
                    ORDER BY CASE WHEN LOWER(name) = LOWER(?2) THEN 0 ELSE 1 END,
                             story_count DESC, name COLLATE NOCASE ASC
                    LIMIT ?3 OFFSET ?4;
                    "#
                };

                let search_input = author_trigram_query.unwrap_or_else(|| author_query.clone());
                let mut stmt = conn.prepare(sql)?;
                let mut total = 0;
                let rows = stmt.query_map(
                    rusqlite::params![search_input, author_query, page_size as i64, offset as i64],
                    |r| {
                        total = r.get::<_, i64>(2).unwrap_or(0) as usize;
                        Ok(AuthorSearchResult {
                            name: r.get(0)?,
                            story_count: r.get(1)?,
                        })
                    },
                )?;
                let list = rows.collect::<rusqlite::Result<Vec<_>>>()?;
                Ok((list, total))
            })
            .await
            .map_err(|e| anyhow!("DB interaction failed: {}", e))??;

        return Ok((SearchResultPayload::Authors(authors), total));
    }

    if raw_q.is_empty() {
        return execute_filter_only_search(conn, query_params, sort_order, offset, page_size).await;
    }

    // Hybrid retrieval: lexical Porter FTS, shared-trigram fuzzy candidates
    // for title/author metadata, semantic vector search, exact-phrase ranking,
    // and a local proximity boost.
    let embeddings = state.model.encode(&[raw_q.to_string()]);
    let raw_embedding = match embeddings.into_iter().next() {
        Some(e) => e,
        None => return Ok((SearchResultPayload::Stories(vec![]), 0)),
    };
    let final_embedding = state.abtt.transform(raw_embedding);
    let embedding_bytes = final_embedding.as_bytes().to_vec();

    let fts_query = apply_scope(&sanitize_fts(raw_q), scope);
    let phrase_query = apply_scope(&sanitize_phrase(raw_q), scope);
    let trigram_query = build_trigram_query(raw_q, scope);
    let proximity_query = build_proximity_boost_query(raw_q).map(|q| apply_scope(&q, scope));
    let has_proximity = proximity_query.is_some();
    let raw_query_for_bonus = raw_q.to_string();

    let and_tags = parse_csv(query_params.and_tags.as_deref());
    let or_tags = parse_csv(query_params.or_tags.as_deref());
    let exc_tags = parse_csv(query_params.exc_tags.as_deref());
    let inc_kws = parse_csv(query_params.inc_kws.as_deref());
    let exc_kws = parse_csv(query_params.exc_kws.as_deref());

    let use_trigram = scope != SearchScope::Content && trigram_query.is_some();
    let max_score = max_score_for_scope(scope, has_proximity, use_trigram);
    let use_vector = scope != SearchScope::Title;
    let use_title_bonus = scope != SearchScope::Content;

    let (results, total) = conn
        .interact(move |conn| -> Result<(Vec<StorySummary>, usize)> {
            let mut sql_params: Vec<Box<dyn ToSql>> = vec![
                Box::new(fts_query),
                Box::new(trigram_query.unwrap_or_else(|| "\"__no_trigram__\"".to_string())),
                Box::new(embedding_bytes),
                Box::new(proximity_query.unwrap_or_else(|| "\"__no_proximity__\"".to_string())),
                Box::new(raw_query_for_bonus),
                Box::new(phrase_query),
            ];

            // Positive tag gates are applied before each ranker's LIMIT so each
            // ranker contributes its best N results *within the selected tag pool*.
            // The complete filter set is still applied afterwards.
            let has_positive_tag_gate = !and_tags.is_empty() || !or_tags.is_empty();
            let tag_gate_where = if has_positive_tag_gate {
                build_where_filters(
                    &and_tags,
                    &or_tags,
                    &[],
                    &[],
                    &[],
                    &mut sql_params,
                )
            } else {
                String::new()
            };
            let where_clause = build_where_filters(
                &and_tags,
                &or_tags,
                &exc_tags,
                &inc_kws,
                &exc_kws,
                &mut sql_params,
            );

            let tag_pool_cte = if has_positive_tag_gate {
                format!(
                    "tag_pool AS (SELECT m.id AS story_id FROM stories_metadata m {}),",
                    tag_gate_where
                )
            } else {
                String::new()
            };
            let fts_pool_join = if has_positive_tag_gate {
                "JOIN tag_pool tp ON tp.story_id = stories_fts.rowid"
            } else {
                ""
            };
            let trigram_pool_join = if has_positive_tag_gate {
                "JOIN tag_pool tp ON tp.story_id = trigram_fts.rowid"
            } else {
                ""
            };

            let trigram_cte = if use_trigram {
                format!(
                    "SELECT trigram_fts.rowid AS story_id FROM trigram_fts {} WHERE trigram_fts MATCH ?2 ORDER BY rank LIMIT 250",
                    trigram_pool_join
                )
            } else {
                "SELECT NULL AS story_id WHERE 0".to_string()
            };
            let vector_cte = if use_vector && has_positive_tag_gate {
                // vec0 KNN LIMIT is global. For a gated search, explicitly score
                // vectors from the tag pool so the 400-vector candidate budget is
                // spent inside the gate rather than filtered after global KNN.
                "SELECT vec_stories.story_id FROM vec_stories JOIN tag_pool tp ON tp.story_id = vec_stories.story_id ORDER BY vec_distance_cosine(vec_stories.embedding, ?3) LIMIT 400".to_string()
            } else if use_vector {
                "SELECT story_id FROM vec_stories WHERE embedding MATCH ?3 ORDER BY distance LIMIT 400".to_string()
            } else {
                "SELECT NULL AS story_id WHERE 0".to_string()
            };
            let proximity_cte = if has_proximity {
                format!(
                    "SELECT stories_fts.rowid AS story_id FROM stories_fts {} WHERE stories_fts MATCH ?4 ORDER BY bm25(stories_fts) LIMIT 300",
                    fts_pool_join
                )
            } else {
                "SELECT NULL AS story_id WHERE 0".to_string()
            };

            sql_params.push(Box::new(page_size as i64));
            sql_params.push(Box::new(offset as i64));
            let limit_idx = sql_params.len() - 1;
            let offset_idx = sql_params.len();

            let order_by = match sort_order {
                SearchSort::Relevance => "ORDER BY final_score DESC, COALESCE(readers, 0) DESC",
                SearchSort::Newest => "ORDER BY timestamp DESC",
                SearchSort::Oldest => "ORDER BY timestamp ASC",
                SearchSort::Title => "ORDER BY COALESCE(NULLIF(TRIM(title), ''), '(not title)') COLLATE NOCASE ASC",
            };

            let title_bonus_sql = if use_title_bonus {
                format!(
                    "CASE WHEN LOWER(m.title) = LOWER(?5) THEN {exact} WHEN LOWER(m.title) LIKE LOWER(?5 || '%') THEN {prefix} ELSE 0.0 END",
                    exact = EXACT_TITLE_BONUS,
                    prefix = EXACT_TITLE_BONUS * 0.25
                )
            } else {
                "0.0".to_string()
            };

            let sql = format!(
                r#"
                WITH
                {tag_pool_cte}
                porter_raw AS (
                    SELECT stories_fts.rowid AS story_id
                    FROM stories_fts
                    {fts_pool_join}
                    WHERE stories_fts MATCH ?1
                    ORDER BY bm25(stories_fts, 12.0, 4.0, 1.0)
                    LIMIT 1500
                ),
                porter_candidates AS (
                    SELECT story_id, ROW_NUMBER() OVER () AS rank_num FROM porter_raw
                ),
                trigram_raw AS (
                    {trigram_cte}
                ),
                trigram_candidates AS (
                    SELECT story_id, ROW_NUMBER() OVER () AS rank_num FROM trigram_raw
                ),
                vec_raw AS (
                    {vector_cte}
                ),
                vec_candidates AS (
                    SELECT story_id, ROW_NUMBER() OVER () AS rank_num FROM vec_raw
                ),
                proximity_raw AS (
                    {proximity_cte}
                ),
                proximity_candidates AS (
                    SELECT story_id, ROW_NUMBER() OVER () AS rank_num FROM proximity_raw
                ),
                phrase_raw AS (
                    SELECT stories_fts.rowid AS story_id
                    FROM stories_fts
                    {fts_pool_join}
                    WHERE stories_fts MATCH ?6
                    ORDER BY bm25(stories_fts, 14.0, 5.0, 1.0)
                    LIMIT 500
                ),
                phrase_candidates AS (
                    SELECT story_id, ROW_NUMBER() OVER () AS rank_num FROM phrase_raw
                ),
                combined_scores AS (
                    SELECT story_id, ({porter_weight} / (60.0 + rank_num)) AS score FROM porter_candidates
                    UNION ALL
                    SELECT story_id, ({trigram_weight} / (20.0 + rank_num)) AS score FROM trigram_candidates
                    UNION ALL
                    SELECT story_id, ({vector_weight} / (60.0 + rank_num)) AS score FROM vec_candidates
                    UNION ALL
                    SELECT story_id, ({proximity_weight} / (60.0 + rank_num)) AS score FROM proximity_candidates
                    UNION ALL
                    SELECT story_id, ({phrase_weight} / (40.0 + rank_num)) AS score FROM phrase_candidates
                ),
                aggregated AS (
                    SELECT cs.story_id, SUM(cs.score) + {title_bonus_sql} AS final_score
                    FROM combined_scores cs
                    JOIN stories_metadata m ON m.id = cs.story_id
                    GROUP BY cs.story_id
                ),
                filtered_matches AS (
                    SELECT m.id, m.title, a.name AS author, COALESCE(m.introduction, '') AS introduction,
                           agg.final_score, m.readers, m.timestamp,
                           strftime('%Y-%m-%d', datetime(m.timestamp, 'unixepoch')) AS date_str,
                           (SELECT GROUP_CONCAT(t.name, ', ') FROM story_tags st JOIN tags t ON st.tag_id = t.id WHERE st.story_id = m.id) AS tags,
                           (SELECT GROUP_CONCAT(k.name, ', ') FROM story_keywords sk JOIN keywords k ON sk.keyword_id = k.id WHERE sk.story_id = m.id) AS keywords,
                           COUNT(*) OVER() AS total_results
                    FROM aggregated agg
                    JOIN stories_metadata m ON agg.story_id = m.id
                    JOIN authors a ON m.author_id = a.id
                    {where_clause}
                )
                SELECT id, title, author, introduction, final_score, readers, timestamp,
                       date_str, tags, keywords, total_results
                FROM filtered_matches
                {order_by}
                LIMIT ?{limit_idx} OFFSET ?{offset_idx};
                "#,
                porter_weight = PORTER_WEIGHT,
                trigram_weight = TRIGRAM_WEIGHT,
                vector_weight = VECTOR_WEIGHT,
                proximity_weight = PROXIMITY_WEIGHT,
                phrase_weight = PHRASE_WEIGHT,
            );

            let mut stmt = conn.prepare(&sql)?;
            let mut total_matches = 0;
            let rows = stmt.query_map(
                rusqlite::params_from_iter(sql_params.iter().map(|b| b.as_ref())),
                |r| {
                    total_matches = r.get::<_, i64>(10).unwrap_or(0) as usize;
                    let raw_score: f32 = r.get(4)?;
                    let scaled_score = Some((raw_score / max_score).clamp(0.0, 1.0));
                    Ok(StorySummary {
                        id: r.get(0)?,
                        title: r.get(1)?,
                        author: r.get(2)?,
                        introduction: r.get(3)?,
                        score: scaled_score,
                        readers: r.get(5)?,
                        date: r.get(7)?,
                        tags: r.get(8)?,
                        keywords: r.get(9)?,
                    })
                },
            )?;
            let list = rows.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok((list, total_matches))
        })
        .await
        .map_err(|e| anyhow!("DB error: {}", e))??;

    Ok((SearchResultPayload::Stories(results), total))
}
