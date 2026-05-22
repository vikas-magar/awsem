use crate::AppState;
use actix_web::{HttpResponse, web};
use rusqlite::params;
use serde_json::{Value, json};

pub async fn create(state: web::Data<AppState>, body: bytes::Bytes) -> HttpResponse {
    let input: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return awsem_core::error::AwsemError::InvalidRequest(e.to_string()).to_response();
        }
    };
    let name = input.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let namespace = input
        .pointer("/containerProvider/info/eksInfo/namespace")
        .and_then(|v| v.as_str())
        .unwrap_or(&state.namespace);
    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:emr-containers:us-east-1:000000000000:/virtualclusters/{id}");
    let c = awsem_core::lock_db!(state);
    if let Err(e) = c.execute(
        "INSERT INTO emr_virtual_clusters (id, name, arn, namespace, state) VALUES (?1, ?2, ?3, ?4, 'RUNNING')",
        params![id, name, arn, namespace],
    ) {
        return awsem_core::error::AwsemError::AlreadyExists(e.to_string()).to_response();
    }
    HttpResponse::Ok().json(json!({
        "id": id, "arn": arn, "name": name, "state": "RUNNING",
        "containerProvider": {
            "id": input.get("containerProvider").and_then(|p| p.get("id")).and_then(|v| v.as_str()).unwrap_or(""),
            "type": input.get("containerProvider").and_then(|p| p.get("type")).and_then(|v| v.as_str()).unwrap_or("EKS"),
            "info": { "eksInfo": { "namespace": namespace } }
        }
    }))
}

pub async fn list(state: web::Data<AppState>) -> HttpResponse {
    let c = awsem_core::lock_db!(state);
    let mut stmt = match c.prepare(
        "SELECT id, name, arn, namespace, state, created_at FROM emr_virtual_clusters WHERE state != 'TERMINATED' ORDER BY created_at DESC"
    ) {
        Ok(s) => s,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
    let rows = match stmt.query_map([], |row| {
        Ok(json!({
            "id": row.get::<_, String>(0)?,
            "name": row.get::<_, String>(1)?,
            "arn": row.get::<_, String>(2)?,
            "namespace": row.get::<_, String>(3)?,
            "state": row.get::<_, String>(4)?,
        }))
    }) {
        Ok(r) => r,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
    let mut clusters: Vec<Value> = Vec::new();
    for row in rows {
        match row {
            Ok(v) => clusters.push(v),
            Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
        }
    }
    HttpResponse::Ok().json(json!({"virtualClusters": clusters}))
}

pub async fn delete(state: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let c = awsem_core::lock_db!(state);
    if c.execute(
        "UPDATE emr_virtual_clusters SET state = 'TERMINATED' WHERE id = ?1",
        params![id],
    )
    .is_err()
    {
        return awsem_core::error::AwsemError::NotFound(format!("Cluster {id} not found"))
            .to_response();
    }
    HttpResponse::Ok().json(json!({"id": id}))
}
