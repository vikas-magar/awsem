use crate::AdminState;
use actix_web::{HttpResponse, web};
use serde_json::json;

fn count(c: &rusqlite::Connection, sql: &str) -> i64 {
    c.query_row(sql, [], |r| r.get(0)).unwrap_or(0)
}

pub async fn handle(state: web::Data<AdminState>) -> HttpResponse {
    let c = match state.db.lock() {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().json(json!({"error": e.to_string()})),
    };
    HttpResponse::Ok().json(json!({
        "cognito_users": count(&c, "SELECT COUNT(*) FROM cognito_users"),
        "secrets": count(&c, "SELECT COUNT(*) FROM secrets_secrets WHERE deleted_at IS NULL"),
        "emr_virtual_clusters": count(&c, "SELECT COUNT(*) FROM emr_virtual_clusters WHERE state = 'RUNNING'"),
        "emr_job_runs": count(&c, "SELECT COUNT(*) FROM emr_job_runs"),
        "lambda_functions": count(&c, "SELECT COUNT(*) FROM lambda_functions"),
    }))
}
