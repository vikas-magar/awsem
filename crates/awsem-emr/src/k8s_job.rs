use crate::spawner;
use crate::AppState;
use actix_web::{HttpResponse, web};
use rusqlite::params;
use serde_json::{Value, json};

fn get_vc_info(state: &AppState, vc_id: &str) -> Option<(String, String)> {
    let c = state.db.lock().ok()?;
    c.query_row(
        "SELECT namespace, name FROM emr_virtual_clusters WHERE id = ?1",
        params![vc_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )
    .ok()
}

#[tracing::instrument(skip(state, body))]
pub async fn start_job_run(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: bytes::Bytes,
) -> HttpResponse {
    let vc_id = path.into_inner();
    let input: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return awsem_core::error::AwsemError::InvalidRequest(e.to_string()).emr_response();
        }
    };
    let name = input.get("name").and_then(|v| v.as_str()).unwrap_or("emr-job");
    let job_driver = input.get("jobDriver").and_then(|v| v.as_object()).cloned().unwrap_or_default();
    let id = uuid::Uuid::new_v4().to_string();
    let arn = state.aws.emr_job_arn(&vc_id, &id);
    let job_driver_json = serde_json::to_string(&job_driver).unwrap_or_default();
    let job_name = format!("emr-job-{id}");
    {
        let c = awsem_core::lock_db!(state);
        if let Err(e) = c.execute(
            "INSERT INTO emr_job_runs (id, name, arn, virtual_cluster_id, job_driver_json, state, kubernetes_job_name) VALUES (?1, ?2, ?3, ?4, ?5, 'SUBMITTED', ?6)",
            params![id, name, arn, vc_id, job_driver_json, job_name],
        ) {
            return awsem_core::error::AwsemError::AlreadyExists(e.to_string()).emr_response();
        }
    }
    let mut job_state = "RUNNING";
    if let (Some(client), Some((vc_ns, _))) = (&state.k8s_client, get_vc_info(&state, &vc_id)) {
        spawner::ensure_rbac(client, &vc_ns).await;
        if let Err(e) = spawner::submit_k8s_job(client, &vc_ns, &id, &job_name, &state.emr_spark_image, &job_driver, &state.awsem_endpoint, &state.rustfs_access_key, &state.rustfs_secret_key).await {
            tracing::error!("Failed to submit K8s job: {e}");
            job_state = "FAILED";
        }
    }
    let s = job_state;
    if let Ok(c) = state.db.lock()
        && let Err(e) = c.execute("UPDATE emr_job_runs SET state = ?1 WHERE id = ?2", params![s, id])
    {
        tracing::error!("Failed to update job state: {e}");
    }
    HttpResponse::Ok().json(json!({
        "id": id, "arn": arn, "name": name, "state": s,
        "virtualClusterId": vc_id,
    }))
}
