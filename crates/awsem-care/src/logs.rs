use crate::AdminState;
use actix_web::{HttpResponse, web};
use serde::Serialize;
use serde_json::json;
use std::fs;

#[derive(Serialize)]
struct LogEntry { timestamp: String, level: String, target: String, message: String, }

fn parse_line(line: &str) -> Option<LogEntry> {
    let line = line.trim();
    if line.is_empty() { return None; }
    let parts: Vec<&str> = line.splitn(4, ' ').collect();
    if parts.len() < 4 { return None; }
    Some(LogEntry { timestamp: parts[0].trim_matches('[').to_string(), level: parts[1].trim().to_string(), target: parts[2].trim().to_string(), message: parts[3..].join(" ") })
}

pub async fn handle(state: web::Data<AdminState>, query: web::Query<std::collections::HashMap<String, String>>) -> HttpResponse {
    let lines: usize = query.get("lines").and_then(|v| v.parse().ok()).unwrap_or(50);
    let filter = query.get("filter").map(String::as_str).unwrap_or("");
    let content = match fs::read_to_string(&state.log_file) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Ok().json(json!({"entries": [], "file": &state.log_file, "error": format!("{e}")})),
    };
    let all: Vec<LogEntry> = content.lines().filter_map(parse_line).filter(|e| filter.is_empty() || e.message.contains(filter) || e.target.contains(filter)).collect();
    let entries = all.into_iter().rev().take(lines).rev().collect::<Vec<_>>();
    HttpResponse::Ok().json(json!({"entries": entries, "file": &state.log_file}))
}
