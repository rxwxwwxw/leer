use anyhow::Result;
use clap::Parser;
use NG_Reader::search::engine::sanitize_fts;
use rusqlite::{params, Connection};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "story-search", about = "Lexical SQL-driven search CLI for stories")]
struct Cli {
    #[arg(short, long)]
    query: String,

    #[arg(short, long)]
    tag: Option<String>,

    #[arg(short, long)]
    author: Option<String>,

    #[arg(short = 'k', long, default_value_t = 20)]
    limit: usize,

    #[arg(long, default_value = "stories.db")]
    db_path: PathBuf,

}

struct SearchResult {
    title: String,
    author: String,
    introduction: String,
    score: f32,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let conn = Connection::open(&args.db_path)?;


    let fts_query = sanitize_fts(&args.query);

    let sql = r#"
    WITH
    porter_raw AS (
        SELECT rowid AS story_id FROM stories_fts WHERE stories_fts MATCH ?1 ORDER BY bm25(stories_fts, 10.0, 20.0, 1.0) LIMIT 1000
        ),
        porter_candidates AS (SELECT story_id, ROW_NUMBER() OVER () AS rank_num FROM porter_raw),
        trigram_raw AS (
            SELECT rowid AS story_id FROM trigram_fts WHERE trigram_fts MATCH ?1 ORDER BY rank LIMIT 100
            ),
            trigram_candidates AS (SELECT story_id, ROW_NUMBER() OVER () AS rank_num FROM trigram_raw),
                combined_scores AS (
                    SELECT story_id, (1.0 / (60.0 + rank_num)) AS score FROM porter_candidates
                    UNION ALL
                    SELECT story_id, (0.3 / (20.0 + rank_num)) AS score FROM trigram_candidates

                    ),
                    aggregated AS (
                        SELECT story_id, SUM(score) AS final_score FROM combined_scores GROUP BY story_id
                        )
                        SELECT m.title, a.name AS author, COALESCE(m.introduction, ''), agg.final_score
                        FROM aggregated agg
                        JOIN stories_metadata m ON agg.story_id = m.id
                        JOIN authors a ON m.author_id = a.id
                        WHERE (?2 IS NULL OR a.name = ?2)
                        AND (?3 IS NULL OR EXISTS (
                            SELECT 1 FROM story_tags st
                            JOIN tags t ON st.tag_id = t.id
                            WHERE st.story_id = m.id AND t.name = ?3
                            ))
                            ORDER BY agg.final_score DESC
                            LIMIT ?4;
                            "#;

                            let mut stmt = conn.prepare(sql)?;
                            let rows = stmt.query_map(
                                params![
                                    fts_query,
                                                      args.author,
                                                      args.tag,
                                                      args.limit as i64
                                ],
                                |r| {
                                    Ok(SearchResult {
                                        title: r.get(0)?,
                                       author: r.get(1)?,
                                       introduction: r.get(2)?,
                                       score: r.get(3)?,
                                    })
                                },
                            )?;

                            println!("\nTop results for: \"{}\"", args.query);
                            println!("{:-<80}", "");
                            for (i, item_res) in rows.enumerate() {
                                let item = item_res?;
                                println!("{}. [{:.5}] \"{}\" by {}", i + 1, item.score, item.title, item.author);
                                if !item.introduction.is_empty() {
                                    let intro_snippet: String = item.introduction.chars().take(120).collect();
                                    println!("   {}...", intro_snippet.trim());
                                }
                            }

                            Ok(())
}
