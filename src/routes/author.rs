use crate::app_state::AppState;
use crate::models::search::Pagination;
use crate::models::story::{
    AuthorDirectoryEntry, AuthorPageQuery, AuthorTagCoverage, AuthorsPageQuery, StorySummary,
};
use crate::templates::views::{author_index_template, authors_index_template};
use actix_web::{get, web, HttpResponse, Responder};
use anyhow::Result;

const AUTHOR_SINGLE_PAGE_LIMIT: usize = 1000;
const AUTHOR_PAGE_SIZE: usize = 1000;
const AUTHORS_DIRECTORY_PAGE_SIZE: usize = 200;

#[get("/author/{author}/")]
pub async fn author_page(
    state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<AuthorPageQuery>,
) -> impl Responder {
    let author = path.into_inner();
    let requested_page = query.page.unwrap_or(1).max(1);
    let conn = match state.stories_pool.get().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB connection error"),
    };

    let author_query = author.clone();
    let result = conn
        .interact(move |conn| -> Result<(Vec<StorySummary>, usize, usize, usize)> {
            let total: usize = conn.query_row(
                r#"
                SELECT COUNT(*)
                FROM stories_metadata m
                JOIN authors a ON m.author_id = a.id
                WHERE a.name = ?1 COLLATE NOCASE
                "#,
                [&author_query],
                |r| r.get::<_, i64>(0),
            )? as usize;

            if total == 0 {
                return Ok((Vec::new(), 0, 1, 1));
            }

            let page_size = if total <= AUTHOR_SINGLE_PAGE_LIMIT {
                total
            } else {
                AUTHOR_PAGE_SIZE
            };
            let total_pages = total.div_ceil(page_size).max(1);
            let current_page = if total <= AUTHOR_SINGLE_PAGE_LIMIT {
                1
            } else {
                requested_page.min(total_pages)
            };
            let offset = (current_page - 1) * page_size;

            let mut stmt = conn.prepare(
                r#"
                SELECT m.id, m.title, a.name, COALESCE(m.introduction, ''), m.readers,
                       strftime('%Y-%m-%d', datetime(m.timestamp, 'unixepoch')),
                       (SELECT GROUP_CONCAT(t.name, ', ') FROM story_tags st JOIN tags t ON st.tag_id = t.id WHERE st.story_id = m.id),
                       (SELECT GROUP_CONCAT(k.name, ', ') FROM story_keywords sk JOIN keywords k ON sk.keyword_id = k.id WHERE sk.story_id = m.id)
                FROM stories_metadata m
                JOIN authors a ON m.author_id = a.id
                WHERE a.name = ?1 COLLATE NOCASE
                ORDER BY COALESCE(NULLIF(TRIM(m.title), ''), '(not title)') COLLATE NOCASE ASC,
                         m.id ASC
                LIMIT ?2 OFFSET ?3
                "#,
            )?;

            let rows = stmt.query_map(
                rusqlite::params![author_query, page_size as i64, offset as i64],
                |r| {
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

            let stories = rows.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok((stories, total, current_page, total_pages))
        })
        .await;

    match result {
        Ok(Ok((stories, total, current_page, total_pages))) => {
            let pagination = Pagination {
                current_page,
                total_pages,
                has_previous: current_page > 1,
                has_next: current_page < total_pages,
            };
            let markup = author_index_template(
                &state.base_path,
                &author,
                &stories,
                total,
                &pagination,
            );
            HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(markup.into_string())
        }
        _ => HttpResponse::InternalServerError().body("Failed to load author"),
    }
}

#[get("/authors")]
pub async fn authors_page(
    state: web::Data<AppState>,
    query: web::Query<AuthorsPageQuery>,
) -> impl Responder {
    let requested_page = query.page.unwrap_or(1).max(1);
    let name_query = query.query.as_deref().unwrap_or("").trim().to_string();
    let tag_coverage = query.tag_coverage.unwrap_or_default();
    let min_stories = query.min_stories.unwrap_or(1).max(1);

    let conn = match state.stories_pool.get().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB connection error"),
    };

    let result = conn
        .interact(move |conn| -> Result<(Vec<AuthorDirectoryEntry>, usize, usize)> {
            let coverage_predicate = match tag_coverage {
                AuthorTagCoverage::All => "1 = 1",
                AuthorTagCoverage::Any => "tagged_story_count > 0",
                AuthorTagCoverage::None => "tagged_story_count = 0",
                AuthorTagCoverage::AllTagged => "tagged_story_count = story_count",
                AuthorTagCoverage::Mixed => {
                    "tagged_story_count > 0 AND tagged_story_count < story_count"
                }
            };

            let stats_cte = format!(
                r#"
                WITH author_stats AS (
                    SELECT a.id, a.name,
                           COUNT(m.id) AS story_count,
                           SUM(CASE WHEN EXISTS (
                               SELECT 1 FROM story_tags st WHERE st.story_id = m.id
                           ) THEN 1 ELSE 0 END) AS tagged_story_count
                    FROM authors a
                    JOIN stories_metadata m ON m.author_id = a.id
                    WHERE (?1 = '' OR LOWER(a.name) LIKE '%' || LOWER(?1) || '%')
                    GROUP BY a.id, a.name
                ),
                filtered_authors AS (
                    SELECT name, story_count, tagged_story_count
                    FROM author_stats
                    WHERE story_count >= ?2 AND {coverage_predicate}
                )
                "#
            );

            let count_sql = format!("{} SELECT COUNT(*) FROM filtered_authors", stats_cte);
            let total: usize = conn.query_row(
                &count_sql,
                rusqlite::params![name_query, min_stories as i64],
                |r| r.get::<_, i64>(0),
            )? as usize;

            let total_pages = total.div_ceil(AUTHORS_DIRECTORY_PAGE_SIZE).max(1);
            let current_page = requested_page.min(total_pages);
            let offset = (current_page - 1) * AUTHORS_DIRECTORY_PAGE_SIZE;

            let page_sql = format!(
                r#"
                {stats_cte}
                SELECT name, story_count, tagged_story_count
                FROM filtered_authors
                ORDER BY name COLLATE NOCASE ASC
                LIMIT ?3 OFFSET ?4
                "#
            );
            let mut stmt = conn.prepare(&page_sql)?;
            let rows = stmt.query_map(
                rusqlite::params![
                    name_query,
                    min_stories as i64,
                    AUTHORS_DIRECTORY_PAGE_SIZE as i64,
                    offset as i64
                ],
                |r| {
                    Ok(AuthorDirectoryEntry {
                        name: r.get(0)?,
                        story_count: r.get(1)?,
                        tagged_story_count: r.get(2)?,
                    })
                },
            )?;
            let authors = rows.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok((authors, total, current_page))
        })
        .await;

    match result {
        Ok(Ok((authors, total, current_page))) => {
            let total_pages = total.div_ceil(AUTHORS_DIRECTORY_PAGE_SIZE).max(1);
            let pagination = Pagination {
                current_page,
                total_pages,
                has_previous: current_page > 1,
                has_next: current_page < total_pages,
            };
            let markup = authors_index_template(
                &state.base_path,
                &query,
                &authors,
                total,
                &pagination,
            );
            HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(markup.into_string())
        }
        _ => HttpResponse::InternalServerError().body("Failed to load authors"),
    }
}
