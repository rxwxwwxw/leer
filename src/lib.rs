pub mod app_state;
pub mod db;
pub mod embeddings;
pub mod models;
pub mod routes;
pub mod search;
pub mod templates;
pub mod utils;

use rusqlite::ffi::sqlite3_auto_extension;
use sqlite_vec::sqlite3_vec_init;

/// Initializes SQLite vector extension globally for the process.
pub fn init_sqlite_extensions() {
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite3_vec_init as *const (),
        )));
    }
}
