use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub type DbConn = Arc<Mutex<Connection>>;

pub fn open(path: &str) -> Result<DbConn, Box<dyn std::error::Error>> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    Ok(Arc::new(Mutex::new(conn)))
}

pub fn init_schema(conn: &DbConn) -> Result<(), Box<dyn std::error::Error>> {
    let sql = include_str!("../schema.sql");
    conn.lock().unwrap().execute_batch(sql)?;
    Ok(())
}
