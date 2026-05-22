use crate::AdminState;
use actix_web::{HttpResponse, web};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Serialize)]
struct UserRow { username: String, status: String, email: Option<String>, created_at: String, }

pub async fn list(state: web::Data<AdminState>) -> HttpResponse {
    let c = match state.db.lock() { Ok(c) => c, Err(e) => return HttpResponse::InternalServerError().json(json!({"error": e.to_string()})), };
    let mut stmt = match c.prepare("SELECT username, status, email, created_at FROM cognito_users ORDER BY created_at DESC") { Ok(s) => s, Err(e) => return HttpResponse::InternalServerError().json(json!({"error": e.to_string()})), };
    let users: Vec<UserRow> = stmt.query_map([], |row| Ok(UserRow { username: row.get(0)?, status: row.get(1)?, email: row.get(2)?, created_at: row.get::<_, String>(3).unwrap_or_default(), })).into_iter().flatten().filter_map(|r| r.ok()).collect();
    HttpResponse::Ok().json(json!({"users": users}))
}

pub async fn delete(state: web::Data<AdminState>, body: bytes::Bytes) -> HttpResponse {
    let input: Value = serde_json::from_slice(&body).unwrap_or_default();
    let username = input.get("username").and_then(|v| v.as_str()).unwrap_or("");
    if username.is_empty() { return HttpResponse::BadRequest().json(json!({"error": "missing username"})); }
    let c = match state.db.lock() { Ok(c) => c, Err(e) => return HttpResponse::InternalServerError().json(json!({"error": e.to_string()})), };
    if let Err(e) = c.execute("DELETE FROM cognito_users WHERE username = ?1", rusqlite::params![username]) { return HttpResponse::NotFound().json(json!({"error": e.to_string()})); }
    HttpResponse::Ok().json(json!({"deleted": username}))
}
