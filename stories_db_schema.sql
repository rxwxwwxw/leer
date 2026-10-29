CREATE TABLE authors(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT UNIQUE NOT NULL
);
CREATE TABLE sqlite_sequence(name,seq);
CREATE TABLE tags(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT UNIQUE NOT NULL
);
CREATE TABLE keywords(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT UNIQUE NOT NULL
);
CREATE TABLE stories_metadata(
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
CREATE TABLE stories_content(
  story_id INTEGER PRIMARY KEY,
  content TEXT NOT NULL
);
CREATE TABLE story_tags(
  story_id INTEGER NOT NULL,
  tag_id INTEGER NOT NULL,
  sample_key INTEGER NOT NULL,
  PRIMARY KEY(story_id, tag_id)
);
CREATE TABLE story_keywords(
  story_id INTEGER NOT NULL,
  keyword_id INTEGER NOT NULL,
  sample_key INTEGER NOT NULL,
  PRIMARY KEY(story_id, keyword_id)
);
CREATE VIEW stories_view AS
        SELECT
        m.id,
        m.title,
        a.name AS author,
        c.content
        FROM stories_metadata m
        JOIN authors a ON m.author_id = a.id
        JOIN stories_content c ON m.id = c.story_id
/* stories_view(id,title,author,content) */;
CREATE VIRTUAL TABLE stories_fts USING fts5(
  title,
  author,
  content,
  content='stories_view',
  content_rowid='id',
  tokenize = 'porter unicode61'
)
/* stories_fts(
  title,
  author,
  content
) */;
CREATE TABLE 'stories_fts_data'(id INTEGER PRIMARY KEY, block BLOB);
CREATE TABLE 'stories_fts_idx'(
  segid,
  term,
  pgno,
  PRIMARY KEY(segid, term)
) WITHOUT ROWID;
CREATE TABLE 'stories_fts_docsize'(id INTEGER PRIMARY KEY, sz BLOB);
CREATE TABLE 'stories_fts_config'(k PRIMARY KEY, v) WITHOUT ROWID;
CREATE VIRTUAL TABLE trigram_fts USING fts5(
  title,
  author,
  tokenize = 'trigram'
)
/* trigram_fts(
  title,
  author
) */;
CREATE TABLE 'trigram_fts_data'(id INTEGER PRIMARY KEY, block BLOB);
CREATE TABLE 'trigram_fts_idx'(
  segid,
  term,
  pgno,
  PRIMARY KEY(segid, term)
) WITHOUT ROWID;
CREATE TABLE 'trigram_fts_content'(id INTEGER PRIMARY KEY, c0, c1);
CREATE TABLE 'trigram_fts_docsize'(id INTEGER PRIMARY KEY, sz BLOB);
CREATE TABLE 'trigram_fts_config'(k PRIMARY KEY, v) WITHOUT ROWID;
CREATE TABLE story_related (
  story_id INTEGER NOT NULL,
  related_id INTEGER NOT NULL,
  score REAL NOT NULL,
  position INTEGER NOT NULL,
  PRIMARY KEY(story_id, related_id)
);
CREATE INDEX idx_story_tags_tag_id ON story_tags(tag_id, story_id);
CREATE INDEX idx_story_keywords_kw_id ON story_keywords(keyword_id, story_id);
CREATE INDEX idx_stories_metadata_author ON stories_metadata(author_id);

CREATE INDEX idx_story_tags_sample ON story_tags(tag_id, sample_key, story_id);
CREATE INDEX idx_story_keywords_sample ON story_keywords(keyword_id, sample_key, story_id);
CREATE INDEX idx_stories_title_sort ON stories_metadata(COALESCE(NULLIF(TRIM(title), ''), '(not title)') COLLATE NOCASE, id);
