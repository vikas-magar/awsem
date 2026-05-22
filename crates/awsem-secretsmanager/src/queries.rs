use crate::store::{Secret, SecretVersion};
use awsem_core::db::DbConn;

pub fn get_secret_by_name(conn: &DbConn, name: &str) -> Result<Secret, String> {
    let c = conn.lock().map_err(|e| e.to_string())?;
    c.query_row(
        "SELECT id, name, arn, description, kms_key_id, tags_json, created_at, last_changed, deleted_at FROM secrets_secrets WHERE name = ?1",
        rusqlite::params![name],
        |row| Ok(Secret {
            id: row.get(0)?, name: row.get(1)?, arn: row.get(2)?,
            description: row.get(3)?, kms_key_id: row.get(4)?,
            tags_json: row.get(5)?, created_at: row.get(6)?,
            last_changed: row.get(7)?, deleted_at: row.get(8)?,
        }),
    ).map_err(|e| e.to_string())
}

pub fn get_secret_by_arn(conn: &DbConn, arn: &str) -> Result<Secret, String> {
    let c = conn.lock().map_err(|e| e.to_string())?;
    c.query_row(
        "SELECT id, name, arn, description, kms_key_id, tags_json, created_at, last_changed, deleted_at FROM secrets_secrets WHERE arn = ?1",
        rusqlite::params![arn],
        |row| Ok(Secret {
            id: row.get(0)?, name: row.get(1)?, arn: row.get(2)?,
            description: row.get(3)?, kms_key_id: row.get(4)?,
            tags_json: row.get(5)?, created_at: row.get(6)?,
            last_changed: row.get(7)?, deleted_at: row.get(8)?,
        }),
    ).map_err(|e| e.to_string())
}

pub fn get_latest_version(conn: &DbConn, secret_id: &str) -> Result<SecretVersion, String> {
    let c = conn.lock().map_err(|e| e.to_string())?;
    c.query_row(
        "SELECT version_id, value FROM secrets_versions WHERE secret_id = ?1 ORDER BY created_at DESC LIMIT 1",
        rusqlite::params![secret_id],
        |row| Ok(SecretVersion { version_id: row.get(0)?, value: row.get(1)? }),
    ).map_err(|e| e.to_string())
}

pub fn list_secrets(conn: &DbConn) -> Result<Vec<Secret>, String> {
    let c = conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = c.prepare(
        "SELECT id, name, arn, description, kms_key_id, tags_json, created_at, last_changed, deleted_at FROM secrets_secrets WHERE deleted_at IS NULL ORDER BY created_at DESC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| Ok(Secret {
        id: row.get(0)?, name: row.get(1)?, arn: row.get(2)?,
        description: row.get(3)?, kms_key_id: row.get(4)?,
        tags_json: row.get(5)?, created_at: row.get(6)?,
        last_changed: row.get(7)?, deleted_at: row.get(8)?,
    })).map_err(|e| e.to_string())?;
    let mut secrets = Vec::new();
    for row in rows {
        secrets.push(row.map_err(|e| e.to_string())?);
    }
    Ok(secrets)
}
