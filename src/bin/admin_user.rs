use anyhow::{bail, Result};
use clap::{Parser, ValueEnum};
use rusqlite::{Connection, OpenFlags};
use NG_Reader::db::{schema::initialize_user_schema, users::UserRepository};

#[derive(Parser)]
struct Args {
    #[arg(value_enum)]
    action: Action,
    username: String,
    #[arg(long, env = "USERS_DB_URL", default_value = "users.db")]
    database: String,
}
#[derive(Clone, ValueEnum)]
enum Action { Promote, Demote }
fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    let args = Args::parse();
    let conn = Connection::open_with_flags(&args.database, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_URI)?;
    initialize_user_schema(&conn)?;
    let promote = matches!(args.action, Action::Promote);
    if !UserRepository::set_admin(&conn, &args.username, promote)? { bail!("User not found"); }
    println!("Admin access {} for {}", if promote { "granted" } else { "revoked" }, args.username);
    Ok(())
}
