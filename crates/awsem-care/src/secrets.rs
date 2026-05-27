use crate::AdminState;
use actix_web::{HttpResponse, web};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Serialize)]
struct SecretRow { name: String, arn: String, description: Option<String>, last_changed: String, }

pub async fn list(state: web::Data<AdminState>) -> HttpResponse {
    let c = match state.db.lock() { Ok(c) => c, Err(e) => return HttpResponse::InternalServerError().json(json!({"error": e.to_string()})), };
    let mut stmt = match c.prepare("SELECT name, arn, description, last_changed FROM secrets_secrets WHERE deleted_at IS NULL ORDER BY last_changed DESC") { Ok(s) => s, Err(e) => return HttpResponse::InternalServerError().json(json!({"error": e.to_string()})), };
    let secrets: Vec<SecretRow> = stmt.query_map([], |row| Ok(SecretRow { name: row.get(0)?, arn: row.get(1)?, description: row.get(2)?, last_changed: row.get::<_, String>(3).unwrap_or_default(), })).into_iter().flatten().filter_map(|r| r.ok()).collect();
    HttpResponse::Ok().json(json!({"secrets": secrets}))
}

pub async fn create(state: web::Data<AdminState>, body: bytes::Bytes) -> HttpResponse {
    let input: Value = serde_json::from_slice(&body).unwrap_or_default();
    let name = input.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let value = input.get("value").and_then(|v| v.as_str()).unwrap_or("");
    let desc = input.get("description").and_then(|v| v.as_str());
    if name.is_empty() { return HttpResponse::BadRequest().json(json!({"error": "missing name"})); }
    let c = match state.db.lock() { Ok(c) => c, Err(e) => return HttpResponse::InternalServerError().json(json!({"error": e.to_string()})), };
    let id = uuid::Uuid::new_v4().to_string();
    let arn = state.aws.secrets_arn(name);
    let now = chrono::Utc::now().to_rfc3339();
    if let Err(e) = c.execute("INSERT INTO secrets_secrets (id, name, arn, description, tags_json, created_at, last_changed) VALUES (?1, ?2, ?3, ?4, '[]', ?5, ?5)", rusqlite::params![id, name, arn, desc, now]) { return HttpResponse::Conflict().json(json!({"error": e.to_string()})); }
    let vid = uuid::Uuid::new_v4().to_string();
    if let Err(e) = c.execute("INSERT INTO secrets_versions (id, secret_id, version_id, value) VALUES (?1, ?2, ?3, ?4)", rusqlite::params![uuid::Uuid::new_v4().to_string(), id, vid, value]) { return HttpResponse::InternalServerError().json(json!({"error": e.to_string()})); }
    HttpResponse::Ok().json(json!({"name": name, "arn": arn, "versionId": vid}))
}

pub async fn delete(state: web::Data<AdminState>, body: bytes::Bytes) -> HttpResponse {
    let input: Value = serde_json::from_slice(&body).unwrap_or_default();
    let name = input.get("name").and_then(|v| v.as_str()).unwrap_or("");
    if name.is_empty() { return HttpResponse::BadRequest().json(json!({"error": "missing name"})); }
    let c = match state.db.lock() { Ok(c) => c, Err(e) => return HttpResponse::InternalServerError().json(json!({"error": e.to_string()})), };
    let now = chrono::Utc::now().to_rfc3339();
    if let Err(e) = c.execute("UPDATE secrets_secrets SET deleted_at = ?1 WHERE name = ?2 AND deleted_at IS NULL", rusqlite::params![now, name]) { return HttpResponse::NotFound().json(json!({"error": e.to_string()})); }
    HttpResponse::Ok().json(json!({"deleted": name}))
}
