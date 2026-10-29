use anyhow::{bail, Context, Result};
use chrono::{DateTime, NaiveDate, NaiveDateTime};
use clap::Parser;
use rusqlite::{params, Connection};
use serde::Deserialize;
use std::{fs::File, io::{BufRead, BufReader}, path::PathBuf};
use NG_Reader::db::schema::initialize_schema;
use NG_Reader::utils::sample_key;

#[derive(Parser)]
struct Args {
    #[arg(default_value = "TEST3.jsonl")]
    input: PathBuf,
    #[arg(default_value = "stories.db")]
    database: PathBuf,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Story {
    title: String, author: String, story: String, tags: String,
    introduction: String, keywords: String, date: String,
    #[serde(rename = "nearest")]
    nearest: Vec<Neighbor>,
}
#[derive(Deserialize)]
struct Neighbor { id: usize, score: f64 }

fn date_timestamp(date: &str) -> Option<i64> {
    let date = date.trim();
    if let Ok(value) = date.parse::<f64>() {
        if value.is_finite() { return Some(value as i64); }
    }
    for format in ["%Y-%m-%d", "%d-%m-%Y", "%m-%d-%Y", "%d/%m/%Y", "%m/%d/%Y", "%Y/%m/%d", "%d %b %Y", "%B %d, %Y", "%b %d, %Y"] {
        if let Ok(parsed) = NaiveDate::parse_from_str(date, format) {
            return parsed.and_hms_opt(0, 0, 0).map(|d| d.and_utc().timestamp());
        }
    }
    if let Ok(parsed) = DateTime::parse_from_rfc3339(date) { return Some(parsed.timestamp()); }
    for format in ["%Y-%m-%d %H:%M:%S%z", "%Y-%m-%dT%H:%M:%S%z"] {
        if let Ok(parsed) = DateTime::parse_from_str(date, format) { return Some(parsed.timestamp()); }
    }
    for format in ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M", "%b %d, %Y %H:%M"] {
        if let Ok(parsed) = NaiveDateTime::parse_from_str(date, format) { return Some(parsed.and_utc().timestamp()); }
    }
    None
}
fn main() -> Result<()> {
    let args = Args::parse();
    let reader = BufReader::new(File::open(&args.input).context("opening input")?);
    let mut conn = Connection::open(&args.database)?;
    initialize_schema(&conn)?;
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM stories_metadata", [], |r| r.get(0))?;
    if count != 0 { bail!("destination database already contains stories"); }
    let tx = conn.transaction()?;
    let mut relations = Vec::new();
    let mut ids = Vec::new();
    for (line, data) in reader.lines().enumerate() {
        let data = data.with_context(|| format!("reading line {}", line + 1))?;
        if data.trim().is_empty() { continue; }
        let record: Story = serde_json::from_str(&data).with_context(|| format!("parsing line {}", line + 1))?;
        tx.execute("INSERT INTO authors(name) VALUES (?1) ON CONFLICT(name) DO NOTHING", [&record.author])?;
        let author: i64 = tx.query_row("SELECT id FROM authors WHERE name=?1", [&record.author], |r| r.get(0))?;
        tx.execute("INSERT INTO stories_metadata(line_index, author_id, title, introduction, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)", params![ids.len() as i64, author, record.title, record.introduction, date_timestamp(&record.date)])?;
        let id = tx.last_insert_rowid();
        ids.push(id);
        tx.execute("INSERT INTO stories_content(story_id, content) VALUES (?1, ?2)", params![id, record.story])?;
        tx.execute("INSERT INTO trigram_fts(rowid, title, author) VALUES (?1, ?2, ?3)", params![id, record.title, record.author])?;
        for (values, table, join, column) in [(&record.tags, "tags", "story_tags", "tag_id"), (&record.keywords, "keywords", "story_keywords", "keyword_id")] {
            for value in values.split(',').map(str::trim).filter(|v| !v.is_empty()) {
                tx.execute(&format!("INSERT INTO {table}(name) VALUES (?1) ON CONFLICT(name) DO NOTHING"), [value])?;
                let item: i64 = tx.query_row(&format!("SELECT id FROM {table} WHERE name=?1"), [value], |r| r.get(0))?;
                tx.execute(&format!("INSERT OR IGNORE INTO {join}(story_id, {column}, sample_key) VALUES (?1, ?2, ?3)"), params![id, item, sample_key(id as u64)])?;
            }
        }
        relations.push((line + 1, record.nearest));
    }
    for (index, (line, neighbors)) in relations.iter().enumerate() {
        for (position, neighbor) in neighbors.iter().enumerate() {
            let related = ids.get(neighbor.id).with_context(|| format!("invalid neighbor {} at line {}", neighbor.id, line))?;
            tx.execute("INSERT INTO story_related(story_id, related_id, score, position) VALUES (?1, ?2, ?3, ?4)", params![ids[index], related, neighbor.score, position as i64]).with_context(|| format!("invalid neighbor at line {line}"))?;
        }
    }
    tx.execute("INSERT INTO stories_fts(stories_fts) VALUES('rebuild')", [])?;
    tx.commit()?;
    conn.execute_batch("ANALYZE; PRAGMA optimize;")?;
    println!("Imported {} stories", ids.len());
    Ok(())
}
