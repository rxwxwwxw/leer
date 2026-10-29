use crate::models::story::{StoryDetail, StorySummary, TagCount};
use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;

pub struct StoryRepository;

impl StoryRepository {
    const SUMMARY_SELECT: &'static str = r#"
    SELECT m.id, m.title, a.name, COALESCE(m.introduction, ''), m.readers,
    strftime('%Y-%m-%d', datetime(m.timestamp, 'unixepoch')),
    (SELECT GROUP_CONCAT(t.name, ', ') FROM story_tags st JOIN tags t ON st.tag_id = t.id WHERE st.story_id = m.id),
    (SELECT GROUP_CONCAT(k.name, ', ') FROM story_keywords sk JOIN keywords k ON sk.keyword_id = k.id WHERE sk.story_id = m.id)
    FROM stories_metadata m
    JOIN authors a ON m.author_id = a.id
    "#;

    pub fn fetch_by_ids(conn: &Connection, ids: &[i64]) -> Result<Vec<StorySummary>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("{} WHERE m.id IN ({})", Self::SUMMARY_SELECT, placeholders);

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(ids), |r| {
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
        })?;

        let mut by_id = HashMap::with_capacity(ids.len());
        for row in rows {
            let story = row?;
            by_id.insert(story.id, story);
        }

        // `IN (...)` does not preserve input order. Keep callers' sampled/nearest
        // ordering stable and let individual views explicitly re-sort when desired.
        Ok(ids.iter().filter_map(|id| by_id.remove(id)).collect())
    }

    pub fn get_detail(conn: &Connection, story_id: i64) -> Result<Option<(StoryDetail, Vec<i64>)>> {
        let sql = r#"
        SELECT m.id, m.title, a.name, COALESCE(strftime('%Y-%m-%d', datetime(m.timestamp, 'unixepoch')), 'Unknown date'),
        COALESCE(m.upvotes, 0), COALESCE(m.downvotes, 0), COALESCE(m.readers, 0), c.content, m.nearest_blob,
        (SELECT GROUP_CONCAT(t.name, ', ') FROM story_tags st JOIN tags t ON st.tag_id = t.id WHERE st.story_id = m.id),
        (SELECT GROUP_CONCAT(k.name, ', ') FROM story_keywords sk JOIN keywords k ON sk.keyword_id = k.id WHERE sk.story_id = m.id),
        m.introduction
        FROM stories_metadata m
        JOIN authors a ON m.author_id = a.id
        JOIN stories_content c ON m.id = c.story_id
        WHERE m.id = ?1
        "#;

        let mut stmt = conn.prepare(sql)?;
        let mut rows = stmt.query_map([story_id], |r| {
            let detail = StoryDetail {
                id: r.get(0)?,
                                      title: r.get(1)?,
                                      author: r.get(2)?,
                                      datetime: r.get(3)?,
                                      upvotes: r.get(4)?,
                                      downvotes: r.get(5)?,
                                      readers: r.get(6)?,
                                      content: r.get(7)?,
                                      tags: r.get(9)?,
                                      keywords: r.get(10)?,
                                      introduction: r.get(11)?,
            };
            let blob: Option<Vec<u8>> = r.get(8)?;
            Ok((detail, blob))
        })?;

        if let Some(res) = rows.next() {
            let (detail, blob) = res?;
            let related_ids = blob
            .map(|b| b.chunks_exact(8).map(|c| i64::from_ne_bytes(c.try_into().unwrap())).take(40).collect())
            .unwrap_or_default();
            Ok(Some((detail, related_ids)))
        } else {
            Ok(None)
        }
    }

    pub fn get_popular_tags(conn: &Connection, min_count: i64) -> Result<Vec<TagCount>> {
        let mut stmt = conn.prepare(
            "SELECT t.name, COUNT(st.story_id) AS cnt
            FROM tags t
            LEFT JOIN story_tags st ON t.id = st.tag_id
            GROUP BY t.id
            HAVING cnt >= ?1
            ORDER BY cnt DESC, t.name ASC"
        )?;
        let rows = stmt.query_map([min_count], |r| {
            Ok(TagCount { name: r.get(0)?, count: r.get(1)? })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
    }
}
