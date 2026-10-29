use anyhow::Result;
use rusqlite::Connection;

pub fn initialize_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS authors (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT UNIQUE NOT NULL
    );
    CREATE TABLE IF NOT EXISTS tags (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT UNIQUE NOT NULL
    );
    CREATE TABLE IF NOT EXISTS keywords (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT UNIQUE NOT NULL
    );
    CREATE TABLE IF NOT EXISTS stories_metadata (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        line_index INTEGER UNIQUE NOT NULL,
        author_id INTEGER NOT NULL,
        title TEXT NOT NULL,
        introduction TEXT,
        readers INTEGER,
        upvotes INTEGER,
        downvotes INTEGER,
        timestamp REAL
    );
    CREATE TABLE IF NOT EXISTS stories_content (
        story_id INTEGER PRIMARY KEY,
        content TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS story_tags (
        story_id INTEGER NOT NULL,
        tag_id INTEGER NOT NULL,
        sample_key INTEGER NOT NULL,
        PRIMARY KEY (story_id, tag_id)
    );
    CREATE TABLE IF NOT EXISTS story_keywords (
        story_id INTEGER NOT NULL,
        keyword_id INTEGER NOT NULL,
        sample_key INTEGER NOT NULL,
        PRIMARY KEY (story_id, keyword_id)
    );
    CREATE TABLE IF NOT EXISTS story_related (
        story_id INTEGER NOT NULL,
        related_id INTEGER NOT NULL,
        score REAL NOT NULL,
        position INTEGER NOT NULL,
        PRIMARY KEY (story_id, related_id)
    );

    CREATE INDEX IF NOT EXISTS idx_story_tags_tag_id ON story_tags(tag_id, story_id);
    CREATE INDEX IF NOT EXISTS idx_story_keywords_kw_id ON story_keywords(keyword_id, story_id);
    CREATE INDEX IF NOT EXISTS idx_story_tags_sample ON story_tags(tag_id, sample_key, story_id);
    CREATE INDEX IF NOT EXISTS idx_story_keywords_sample ON story_keywords(keyword_id, sample_key, story_id);
    CREATE INDEX IF NOT EXISTS idx_stories_title_sort ON stories_metadata(COALESCE(NULLIF(TRIM(title), ''), '(not title)') COLLATE NOCASE, id);
    CREATE INDEX IF NOT EXISTS idx_stories_metadata_author ON stories_metadata(author_id);
    CREATE INDEX IF NOT EXISTS idx_stories_metadata_timestamp ON stories_metadata(timestamp);
    CREATE INDEX IF NOT EXISTS idx_stories_metadata_title_nocase ON stories_metadata(title COLLATE NOCASE);
    CREATE INDEX IF NOT EXISTS idx_tags_name_nocase ON tags(name COLLATE NOCASE);
    CREATE INDEX IF NOT EXISTS idx_keywords_name_nocase ON keywords(name COLLATE NOCASE);
    CREATE INDEX IF NOT EXISTS idx_authors_name_nocase ON authors(name COLLATE NOCASE);

    CREATE VIEW IF NOT EXISTS stories_view AS
    SELECT
    m.id,
    m.title,
    a.name AS author,
    c.content
    FROM stories_metadata m
    JOIN authors a ON m.author_id = a.id
    JOIN stories_content c ON m.id = c.story_id;

    CREATE VIRTUAL TABLE IF NOT EXISTS stories_fts USING fts5(
        title,
        author,
        content,
        content='stories_view',
        content_rowid='id',
        tokenize = 'porter unicode61'
    );
    CREATE VIRTUAL TABLE IF NOT EXISTS trigram_fts USING fts5(
        title,
        author,
        tokenize = 'trigram'
    )",
    )?;
    Ok(())
}


pub fn initialize_user_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT UNIQUE NOT NULL COLLATE NOCASE,
            password_hash TEXT NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
    );

    CREATE TABLE IF NOT EXISTS reading_lists (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                       title TEXT NOT NULL,
                       is_public INTEGER NOT NULL DEFAULT 0,
                       created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
    );

    CREATE TABLE IF NOT EXISTS reading_list_items (
        list_id INTEGER NOT NULL REFERENCES reading_lists(id) ON DELETE CASCADE,
                       story_id INTEGER NOT NULL, -- references stories_metadata(id) in stories.db
                       added_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                       notes TEXT,
                       PRIMARY KEY (list_id, story_id)
    );

    CREATE INDEX IF NOT EXISTS idx_lists_user ON reading_lists(user_id);
    CREATE INDEX IF NOT EXISTS idx_list_items_story ON reading_list_items(story_id);",
    )?;
    let has_admin: bool = {
        let mut stmt = conn.prepare("PRAGMA table_info(users)")?;
        let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
        columns.collect::<rusqlite::Result<Vec<_>>>()?.iter().any(|name| name == "is_admin")
    };
    if !has_admin {
        conn.execute("ALTER TABLE users ADD COLUMN is_admin INTEGER NOT NULL DEFAULT 0 CHECK (is_admin IN (0, 1))", [])?;
    }
    Ok(())
}
