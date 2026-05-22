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

pub fn create_secret(
    conn: &DbConn, id: &str, name: &str, arn: &str, secret_string: &str,
    description: Option<&str>, kms_key_id: Option<&str>,
) -> Result<String, String> {
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

pub fn put_secret_value(conn: &DbConn, secret_id: &str, secret_string: &str) -> Result<String, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let version_id = uuid::Uuid::new_v4().to_string();
    let c = conn.lock().map_err(|e| e.to_string())?;
    c.execute(
        "INSERT INTO secrets_versions (id, secret_id, version_id, value, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![uuid::Uuid::new_v4().to_string(), secret_id, version_id, secret_string, now],
    ).map_err(|e| e.to_string())?;
    c.execute("UPDATE secrets_secrets SET last_changed = ?1 WHERE id = ?2", rusqlite::params![now, secret_id])
        .map_err(|e| e.to_string())?;
    Ok(version_id)
}

pub fn soft_delete(conn: &DbConn, secret_id: &str) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let c = conn.lock().map_err(|e| e.to_string())?;
    c.execute("UPDATE secrets_secrets SET deleted_at = ?1 WHERE id = ?2", rusqlite::params![now, secret_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn restore(conn: &DbConn, secret_id: &str) -> Result<(), String> {
    let c = conn.lock().map_err(|e| e.to_string())?;
    c.execute("UPDATE secrets_secrets SET deleted_at = NULL WHERE id = ?1", rusqlite::params![secret_id])
        .map_err(|e| e.to_string())?;
    Ok(())
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
