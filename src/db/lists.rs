// src/db/lists.rs
use anyhow::Result;
use rusqlite::{params, Connection};
use crate::models::list::ReadingList;

pub struct ListRepository;

impl ListRepository {
    pub fn create_list(conn: &Connection, user_id: i64, title: &str, is_public: bool) -> Result<i64> {
        let mut stmt = conn.prepare(
            "INSERT INTO reading_lists (user_id, title, is_public) VALUES (?1, ?2, ?3) RETURNING id",
        )?;
        let id: i64 = stmt.query_row(params![user_id, title, is_public as i32], |r| r.get(0))?;
        Ok(id)
    }

    pub fn get_user_lists(conn: &Connection, user_id: i64) -> Result<Vec<ReadingList>> {
        let mut stmt = conn.prepare(
            "SELECT l.id, l.user_id, l.title, l.is_public, l.created_at, COUNT(li.story_id) AS story_count
            FROM reading_lists l
            LEFT JOIN reading_list_items li ON l.id = li.list_id
            WHERE l.user_id = ?1
            GROUP BY l.id
            ORDER BY l.created_at DESC",
        )?;

        let rows = stmt.query_map([user_id], |r| {
            Ok(ReadingList {
                id: r.get(0)?,
               user_id: r.get(1)?,
               title: r.get(2)?,
               is_public: r.get::<_, i32>(3)? != 0,
               created_at: r.get(4)?,
               story_count: r.get(5)?,
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
    }

    pub fn get_list(conn: &Connection, list_id: i64) -> Result<Option<ReadingList>> {
        let mut stmt = conn.prepare(
            "SELECT l.id, l.user_id, l.title, l.is_public, l.created_at, COUNT(li.story_id) AS story_count
            FROM reading_lists l
            LEFT JOIN reading_list_items li ON l.id = li.list_id
            WHERE l.id = ?1
            GROUP BY l.id",
        )?;

        let mut rows = stmt.query_map([list_id], |r| {
            Ok(ReadingList {
                id: r.get(0)?,
               user_id: r.get(1)?,
               title: r.get(2)?,
               is_public: r.get::<_, i32>(3)? != 0,
               created_at: r.get(4)?,
               story_count: r.get(5)?,
            })
        })?;

        match rows.next() {
            Some(res) => Ok(Some(res?)),
            None => Ok(None),
        }
    }

    pub fn get_list_items(conn: &Connection, list_id: i64) -> Result<Vec<(i64, Option<String>, i64)>> {
        let mut stmt = conn.prepare(
            "SELECT story_id, notes, added_at
            FROM reading_list_items
            WHERE list_id = ?1
            ORDER BY added_at DESC",
        )?;

        let rows = stmt.query_map([list_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
    }

    pub fn add_story(conn: &Connection, list_id: i64, story_id: i64, notes: Option<&str>) -> Result<()> {
        conn.execute(
            "INSERT INTO reading_list_items (list_id, story_id, notes)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(list_id, story_id) DO UPDATE SET notes = excluded.notes",
                     params![list_id, story_id, notes],
        )?;
        Ok(())
    }

    pub fn remove_story(conn: &Connection, list_id: i64, story_id: i64) -> Result<()> {
        conn.execute(
            "DELETE FROM reading_list_items WHERE list_id = ?1 AND story_id = ?2",
            params![list_id, story_id],
        )?;
        Ok(())
    }

    pub fn delete_list(conn: &Connection, list_id: i64, user_id: i64) -> Result<bool> {
        let affected = conn.execute(
            "DELETE FROM reading_lists WHERE id = ?1 AND user_id = ?2",
            params![list_id, user_id],
        )?;
        Ok(affected > 0)
    }
}
