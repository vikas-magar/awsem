use awsem_core::db::DbConn;

pub struct Secret {
    pub id: String,
    pub name: String,
    pub arn: String,
    pub description: Option<String>,
    pub kms_key_id: Option<String>,
    pub tags_json: String,
    pub created_at: String,
    pub last_changed: String,
    pub deleted_at: Option<String>,
}

pub struct SecretVersion {
    pub version_id: String,
    pub value: String,
}

pub fn create_secret(conn: &DbConn, id: &str, name: &str, arn: &str, secret_string: &str, description: Option<&str>, kms_key_id: Option<&str>) -> Result<String, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let version_id = uuid::Uuid::new_v4().to_string();
    let c = conn.lock().map_err(|e| e.to_string())?;
    c.execute(
        "INSERT INTO secrets_secrets (id, name, arn, description, kms_key_id, tags_json, created_at, last_changed) VALUES (?1, ?2, ?3, ?4, ?5, '[]', ?6, ?6)",
        rusqlite::params![id, name, arn, description, kms_key_id, now],
    ).map_err(|e| e.to_string())?;
    c.execute(
        "INSERT INTO secrets_versions (id, secret_id, version_id, value) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![uuid::Uuid::new_v4().to_string(), id, version_id, secret_string],
    ).map_err(|e| e.to_string())?;
    Ok(version_id)
}

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
        |row| Ok(SecretVersion {
            version_id: row.get(0)?, value: row.get(1)?,
        }),
    ).map_err(|e| e.to_string())
}

pub fn put_secret_value(conn: &DbConn, secret_id: &str, secret_string: &str) -> Result<String, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let version_id = uuid::Uuid::new_v4().to_string();
    let c = conn.lock().map_err(|e| e.to_string())?;
    c.execute(
        "INSERT INTO secrets_versions (id, secret_id, version_id, value) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![uuid::Uuid::new_v4().to_string(), secret_id, version_id, secret_string],
    ).map_err(|e| e.to_string())?;
    c.execute(
        "UPDATE secrets_secrets SET last_changed = ?1 WHERE id = ?2",
        rusqlite::params![now, secret_id],
    ).map_err(|e| e.to_string())?;
    Ok(version_id)
}

pub fn soft_delete(conn: &DbConn, secret_id: &str) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let c = conn.lock().map_err(|e| e.to_string())?;
    c.execute(
        "UPDATE secrets_secrets SET deleted_at = ?1 WHERE id = ?2",
        rusqlite::params![now, secret_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn restore(conn: &DbConn, secret_id: &str) -> Result<(), String> {
    let c = conn.lock().map_err(|e| e.to_string())?;
    c.execute(
        "UPDATE secrets_secrets SET deleted_at = NULL WHERE id = ?1",
        rusqlite::params![secret_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_secrets(conn: &DbConn) -> Result<Vec<Secret>, String> {
    let c = conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = c.prepare(
        "SELECT id, name, arn, description, kms_key_id, tags_json, created_at, last_changed, deleted_at FROM secrets_secrets WHERE deleted_at IS NULL ORDER BY created_at DESC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(Secret {
            id: row.get(0)?, name: row.get(1)?, arn: row.get(2)?,
            description: row.get(3)?, kms_key_id: row.get(4)?,
            tags_json: row.get(5)?, created_at: row.get(6)?,
            last_changed: row.get(7)?, deleted_at: row.get(8)?,
        })
    }).map_err(|e| e.to_string())?;
    let mut secrets = Vec::new();
    for row in rows {
        secrets.push(row.map_err(|e| e.to_string())?);
    }
    Ok(secrets)
}

pub fn update_secret(conn: &DbConn, secret_id: &str, description: Option<&str>, kms_key_id: Option<&str>) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let c = conn.lock().map_err(|e| e.to_string())?;
    c.execute(
        "UPDATE secrets_secrets SET description = COALESCE(?1, description), kms_key_id = COALESCE(?2, kms_key_id), last_changed = ?3 WHERE id = ?4",
        rusqlite::params![description, kms_key_id, now, secret_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}
