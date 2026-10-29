use anyhow::{Context, Result};
use NG_Reader::db::schema::initialize_schema;
use NG_Reader::init_sqlite_extensions;
use rusqlite::{params, Connection, Transaction};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use zerocopy::IntoBytes;

#[derive(Debug, Deserialize)]
struct StoryRecord {
    #[serde(rename = "Story")]
    story: String,
    #[serde(rename = "Title")]
    title: String,
    #[serde(rename = "Author")]
    author: String,
    #[serde(rename = "Tags")]
    tags: String,
    #[serde(rename = "Introduction")]
    introduction: String,
    #[serde(rename = "Keywords")]
    keywords: String,
    #[serde(rename = "Readers")]
    readers: f64,
    #[serde(rename = "Upvotes")]
    upvotes: f64,
    #[serde(rename = "Downvotes")]
    downvotes: f64,
    embedding: Vec<f32>,
    #[serde(rename = "Timestamp")]
    timestamp: f64,
    #[serde(rename = "Nearest")]
    nearest: Vec<i64>,
}

fn get_or_create_id(
    cache: &mut HashMap<String, i64>,
    stmt: &mut rusqlite::CachedStatement,
    name: &str,
) -> Result<i64> {
    let clean = name.trim();
    if let Some(&id) = cache.get(clean) {
        return Ok(id);
    }
    let id: i64 = stmt.query_row(params![clean], |row| row.get(0))?;
    cache.insert(clean.to_string(), id);
    Ok(id)
}

fn insert_record(
    tx: &Transaction,
    r: &StoryRecord,
    line_index: i64,
    author_cache: &mut HashMap<String, i64>,
    tag_cache: &mut HashMap<String, i64>,
    keyword_cache: &mut HashMap<String, i64>,
) -> Result<()> {
    let mut stmt_author = tx.prepare_cached(
        "INSERT INTO authors (name) VALUES (?1)
    ON CONFLICT(name) DO UPDATE SET name=excluded.name
    RETURNING id",
    )?;
    let author_id = get_or_create_id(author_cache, &mut stmt_author, &r.author)?;

    let nearest_bytes = r.nearest.as_bytes();
    let mut stmt_meta = tx.prepare_cached(
        "INSERT INTO stories_metadata
        (line_index, author_id, title, introduction, readers, upvotes, downvotes, timestamp, nearest_blob)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
    RETURNING id",
    )?;
    let story_id: i64 = stmt_meta.query_row(
        params![
            line_index,
            author_id,
            r.title,
            r.introduction,
            r.readers as i64,
            r.upvotes as i64,
            r.downvotes as i64,
            r.timestamp,
            nearest_bytes
        ],
        |row| row.get(0),
    )?;

    let mut stmt_content = tx.prepare_cached(
        "INSERT INTO stories_content (story_id, content) VALUES (?1, ?2)",
    )?;
    stmt_content.execute(params![story_id, r.story])?;

    let mut stmt_tag_ins = tx.prepare_cached(
        "INSERT INTO tags (name) VALUES (?1)
    ON CONFLICT(name) DO UPDATE SET name=excluded.name
    RETURNING id",
    )?;
    let mut stmt_st_ins = tx.prepare_cached(
        "INSERT OR IGNORE INTO story_tags (story_id, tag_id) VALUES (?1, ?2)",
    )?;
    for tag in r.tags.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let tag_id = get_or_create_id(tag_cache, &mut stmt_tag_ins, tag)?;
        stmt_st_ins.execute(params![story_id, tag_id])?;
    }

    let mut stmt_kw_ins = tx.prepare_cached(
        "INSERT INTO keywords (name) VALUES (?1)
    ON CONFLICT(name) DO UPDATE SET name=excluded.name
    RETURNING id",
    )?;
    let mut stmt_sk_ins = tx.prepare_cached(
        "INSERT OR IGNORE INTO story_keywords (story_id, keyword_id) VALUES (?1, ?2)",
    )?;
    for kw in r.keywords.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let kw_id = get_or_create_id(keyword_cache, &mut stmt_kw_ins, kw)?;
        stmt_sk_ins.execute(params![story_id, kw_id])?;
    }

    let mut stmt_trigram = tx.prepare_cached(
        "INSERT INTO trigram_fts (rowid, title, author) VALUES (?1, ?2, ?3)",
    )?;
    stmt_trigram.execute(params![story_id, r.title, r.author])?;

    let vector_raw = r.embedding.as_bytes();
    let mut stmt_vec = tx.prepare_cached(
        "INSERT INTO vec_stories (story_id, embedding) VALUES (?1, ?2)",
    )?;
    stmt_vec.execute(params![story_id, vector_raw])?;

    Ok(())
}

pub fn ingest_jsonl<P: AsRef<Path>>(jsonl_path: P, db_path: P) -> Result<()> {
    let mut conn = Connection::open(db_path)?;
    initialize_schema(&conn)?;

    conn.execute_batch(
        "PRAGMA synchronous = OFF;
        PRAGMA cache_size = -2000000;
        PRAGMA temp_store = MEMORY;
        PRAGMA foreign_keys = OFF;",
    )?;

    let file = File::open(jsonl_path)?;
    let reader = BufReader::with_capacity(16 * 1024 * 1024, file);
    let mut author_cache = HashMap::new();
    let mut tag_cache = HashMap::new();
    let mut keyword_cache = HashMap::new();

    const BATCH_SIZE: usize = 5000;
    let mut count: usize = 0;
    let mut line_index: i64 = 1;
    let mut tx = conn.transaction()?;

    for line in reader.lines() {
        let line_str = line?;
        if line_str.trim().is_empty() {
            continue;
        }
        let record: StoryRecord = serde_json::from_str(&line_str)
        .with_context(|| format!("Failed parsing JSON at line {}", line_index))?;

        insert_record(
            &tx,
            &record,
            line_index,
            &mut author_cache,
            &mut tag_cache,
            &mut keyword_cache,
        )?;

        count += 1;
        line_index += 1;

        if count % BATCH_SIZE == 0 {
            tx.commit()?;
            println!("Ingested {} rows...", count);
            tx = conn.transaction()?;
        }
    }
    tx.commit()?;

    println!("Ingestion complete. Rebuilding FTS index...");
    conn.execute("INSERT INTO stories_fts(stories_fts) VALUES('rebuild')", [])?;

    conn.execute_batch(
        "PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;
        PRAGMA optimize;",
    )?;

    Ok(())
}

fn main() -> Result<()> {
    init_sqlite_extensions();
    ingest_jsonl("../Aug26.jsonl", "stories.db")
}
