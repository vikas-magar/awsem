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
    let c = conn.lock().unwrap();
    c.execute_batch(sql).ok();
    // migration: add image column if missing (harmless if already present)
    let _ = c.execute("ALTER TABLE lambda_functions ADD COLUMN image TEXT", []);
    let _ = c.execute("ALTER TABLE emr_job_runs ADD COLUMN logs TEXT", []);
    let _ = c.execute("ALTER TABLE emr_job_runs ADD COLUMN exit_code INTEGER", []);
    Ok(())
}
