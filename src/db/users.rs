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
    pub fn is_admin(conn: &Connection, user_id: i64) -> Result<bool> {
        use rusqlite::OptionalExtension;
        Ok(conn.query_row("SELECT is_admin FROM users WHERE id = ?1", [user_id], |r| r.get::<_, bool>(0)).optional()?.unwrap_or(false))
    }

    pub fn set_admin(conn: &Connection, username: &str, admin: bool) -> Result<bool> {
        Ok(conn.execute("UPDATE users SET is_admin = ?1 WHERE username = ?2 COLLATE NOCASE", params![admin, username])? == 1)
    }

    /// Bootstrap only when no admin exists; never take over an existing account.
    pub fn bootstrap_admin(conn: &Connection, username: &str, password: &str) -> Result<bool> {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM users WHERE is_admin = 1", [], |r| r.get(0))?;
        if count != 0 { return Ok(false); }
        anyhow::ensure!((3..=32).contains(&username.len()) && username == username.trim(), "ADMIN_USERNAME must be 3–32 bytes without surrounding whitespace");
        anyhow::ensure!(password.len() >= 8, "ADMIN_PASSWORD must contain at least 8 bytes");
        anyhow::ensure!(Self::get_by_username(conn, username)?.is_none(), "ADMIN_USERNAME already exists; promote the existing account explicitly");
        let hash = hash_password(password)?;
        conn.execute("INSERT INTO users(username, password_hash, is_admin) VALUES (?1, ?2, 1)", params![username, hash])?;
        Ok(true)
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::initialize_user_schema;

    #[test]
    fn bootstrap_does_not_take_over_users_or_reset_existing_admin() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        initialize_user_schema(&conn)?;
        UserRepository::create_user(&conn, "existing", &hash_password("old-password")?)?;
        assert!(UserRepository::bootstrap_admin(&conn, "existing", "new-password").is_err());
        assert!(!UserRepository::is_admin(&conn, 1)?);
        assert!(UserRepository::bootstrap_admin(&conn, "admin", "initial-password")?);
        assert!(!UserRepository::bootstrap_admin(&conn, "other", "another-password")?);
        assert!(!UserRepository::bootstrap_admin(&conn, "admin", "different-password")?);
        let (_, _, hash) = UserRepository::get_by_username(&conn, "admin")?.unwrap();
        assert!(verify_password("initial-password", &hash));
        assert!(!verify_password("different-password", &hash));
        Ok(())
    }

    #[test]
    fn existing_users_are_not_admin_and_role_can_be_changed() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("CREATE TABLE users (id INTEGER PRIMARY KEY, username TEXT UNIQUE NOT NULL COLLATE NOCASE, password_hash TEXT NOT NULL, created_at INTEGER NOT NULL DEFAULT 0); INSERT INTO users (username, password_hash) VALUES ('alice', 'hash');")?;
        initialize_user_schema(&conn)?;
        initialize_user_schema(&conn)?;
        assert!(!UserRepository::is_admin(&conn, 1)?);
        assert!(!UserRepository::is_admin(&conn, 99)?);
        assert!(UserRepository::set_admin(&conn, "ALICE", true)?);
        assert!(UserRepository::is_admin(&conn, 1)?);
        assert!(UserRepository::set_admin(&conn, "alice", false)?);
        assert!(!UserRepository::is_admin(&conn, 1)?);
        assert!(!UserRepository::set_admin(&conn, "missing", true)?);
        Ok(())
    }
}
