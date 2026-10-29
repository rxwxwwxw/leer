// src/db/users.rs
use anyhow::{anyhow, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rusqlite::{params, Connection};
use crate::models::user::User;

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
    .hash_password(password.as_bytes(), &salt)
    .map(|h| h.to_string())
    .map_err(|e| anyhow!("Failed to hash password: {}", e))
}

pub fn verify_password(password: &str, password_hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
    .verify_password(password.as_bytes(), &parsed_hash)
    .is_ok()
}

pub struct UserRepository;

impl UserRepository {
    pub fn create_user(conn: &Connection, username: &str, password_hash: &str) -> Result<i64> {
        let mut stmt = conn.prepare(
            "INSERT INTO users (username, password_hash) VALUES (?1, ?2) RETURNING id",
        )?;
        let id: i64 = stmt.query_row(params![username, password_hash], |r| r.get(0))?;
        Ok(id)
    }

    pub fn get_by_username(conn: &Connection, username: &str) -> Result<Option<(i64, String, String)>> {
        let mut stmt = conn.prepare(
            "SELECT id, username, password_hash FROM users WHERE username = ?1",
        )?;
        let mut rows = stmt.query_map([username], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?;

        match rows.next() {
            Some(res) => Ok(Some(res?)),
            None => Ok(None),
        }
    }

    pub fn get_by_id(conn: &Connection, user_id: i64) -> Result<Option<User>> {
        let mut stmt = conn.prepare(
            "SELECT id, username, created_at FROM users WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map([user_id], |r| {
            Ok(User {
                id: r.get(0)?,
               username: r.get(1)?,
               created_at: r.get(2)?,
            })
        })?;

        match rows.next() {
            Some(res) => Ok(Some(res?)),
            None => Ok(None),
        }
    }
}
