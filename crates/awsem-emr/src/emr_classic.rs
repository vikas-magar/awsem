use crate::emr_classic_steps;
use crate::AppState;
use actix_web::{HttpRequest, HttpResponse, web};
use chrono::NaiveDateTime;
use rusqlite::params;
use serde_json::{Value, json};

pub fn to_rfc3339(sqlite_ts: &str) -> String {
    NaiveDateTime::parse_from_str(sqlite_ts, "%Y-%m-%d %H:%M:%S")
        .map(|dt| dt.and_utc().to_rfc3339())
        .unwrap_or_else(|_| sqlite_ts.to_string())
}

fn jfid() -> String { format!("j-{}", &uuid::Uuid::new_v4().to_string()[..8]) }

pub async fn handle(state: web::Data<AppState>, req: HttpRequest, body: bytes::Bytes) -> HttpResponse {
    let target = req.headers().get("X-Amz-Target").and_then(|v| v.to_str().ok()).unwrap_or("");
    match target {
        "ElasticMapReduce.RunJobFlow" => run_job_flow(state, body).await,
        "ElasticMapReduce.ListClusters" => list_clusters(state).await,
        "ElasticMapReduce.DescribeCluster" => describe_cluster(state, body).await,
        "ElasticMapReduce.TerminateJobFlows" => terminate_job_flows(state, body).await,
        "ElasticMapReduce.AddJobFlowSteps" => emr_classic_steps::add_job_flow_steps(state, body).await,
        "ElasticMapReduce.ListSteps" => emr_classic_steps::list_steps(state, body).await,
        "ElasticMapReduce.DescribeStep" => emr_classic_steps::describe_step(state, body).await,
        "ElasticMapReduce.SetTerminationProtection" | "ElasticMapReduce.SetVisibleToAllUsers" => HttpResponse::Ok().json(json!({})),
        _ => awsem_core::error::AwsemError::NotImplemented(target.into()).to_response(),
    }
}

async fn run_job_flow(state: web::Data<AppState>, body: bytes::Bytes) -> HttpResponse {
    let input: Value = serde_json::from_slice(&body).unwrap_or_default();
    let name = input.get("Name").and_then(|v| v.as_str()).unwrap_or("emr-cluster");
    let release = input.get("ReleaseLabel").and_then(|v| v.as_str()).unwrap_or("emr-7.1.0-latest");
    let keep_alive = input.pointer("/Instances/KeepJobFlowAliveWhenNoSteps").and_then(|v| v.as_bool()).unwrap_or(true);
    let vc_id = uuid::Uuid::new_v4().to_string();
    let job_flow = jfid();
    {
        let c = awsem_core::lock_db!(state);
        let arn = state.aws.emr_vc_arn(&vc_id);
        if c.execute("INSERT INTO emr_virtual_clusters (id, name, arn, namespace, state) VALUES (?1, ?2, ?3, ?4, 'RUNNING')", params![vc_id, name, arn, state.namespace]).is_err()
            || c.execute("INSERT INTO emr_classic_clusters (job_flow_id, name, virtual_cluster_id, state) VALUES (?1, ?2, ?3, 'RUNNING')", params![job_flow, name, vc_id]).is_err()
        { return awsem_core::error::AwsemError::AlreadyExists(job_flow.clone()).to_response(); }
    }
    if let Some(steps) = input.get("Steps").and_then(|v| v.as_array()) {
        for step in steps {
            let id = uuid::Uuid::new_v4().to_string();
            emr_classic_steps::submit_step(&state, &vc_id, step, &id).await;
        }
    }
    if !keep_alive {
        let c = awsem_core::lock_db!(state);
        let _ = c.execute("UPDATE emr_virtual_clusters SET state = 'TERMINATED' WHERE id = ?1", params![vc_id]);
    }
    HttpResponse::Ok().json(json!({"JobFlowId": job_flow}))
}

async fn list_clusters(state: web::Data<AppState>) -> HttpResponse {
    let c = awsem_core::lock_db!(state);
    let mut stmt = match c.prepare("SELECT job_flow_id, name, state, created_at FROM emr_classic_clusters WHERE state != 'TERMINATED' ORDER BY created_at DESC") {
        Ok(s) => s, Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
    let rows = match stmt.query_map([], |row| Ok(json!({"Id": row.get::<_,String>(0)?, "Name": row.get::<_,String>(1)?, "Status": {"State": row.get::<_,String>(2)?, "Timeline": {"CreationDateTime": to_rfc3339(&row.get::<_,String>(3)?)}}})))
    { Ok(r) => r, Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response() };
    let mut clusters = Vec::new();
    for row in rows { match row { Ok(v) => clusters.push(v), Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response() } }
    HttpResponse::Ok().json(json!({"Clusters": clusters}))
}

async fn describe_cluster(state: web::Data<AppState>, body: bytes::Bytes) -> HttpResponse {
    let input: Value = serde_json::from_slice(&body).unwrap_or_default();
    let jf = input.get("ClusterId").and_then(|v| v.as_str()).unwrap_or("");
    let c = awsem_core::lock_db!(state);
    match c.query_row(
        "SELECT name, state, created_at FROM emr_classic_clusters WHERE job_flow_id = ?1",
        params![jf],
        |row| Ok(json!({"Id": jf, "Name": row.get::<_,String>(0)?, "Status": {"State": row.get::<_,String>(1)?, "Timeline": {"CreationDateTime": to_rfc3339(&row.get::<_,String>(2)?)}}, "NormalizedInstanceHours": "0", "ClusterArn": state.aws.emr_cluster_arn(jf)})),
    ) { Ok(c) => HttpResponse::Ok().json(json!({"Cluster": c})), Err(_) => awsem_core::error::AwsemError::NotFound(jf.into()).to_response() }
}

async fn terminate_job_flows(state: web::Data<AppState>, body: bytes::Bytes) -> HttpResponse {
    let input: Value = serde_json::from_slice(&body).unwrap_or_default();
    for jf in input.get("JobFlowIds").and_then(|v| v.as_array()).into_iter().flatten().filter_map(|x| x.as_str()) {
        let c = awsem_core::lock_db!(state);
        if let Ok(vc_id) = c.query_row("SELECT virtual_cluster_id FROM emr_classic_clusters WHERE job_flow_id = ?1", params![jf], |r| r.get::<_,String>(0)) {
            let _ = c.execute("UPDATE emr_job_runs SET state = 'CANCELLED' WHERE virtual_cluster_id = ?1 AND state NOT IN ('COMPLETED','FAILED','CANCELLED')", params![vc_id]);
            let _ = c.execute("UPDATE emr_virtual_clusters SET state = 'TERMINATED' WHERE id = ?1", params![vc_id]);
            let _ = c.execute("UPDATE emr_classic_clusters SET state = 'TERMINATED', terminated_at = CURRENT_TIMESTAMP WHERE job_flow_id = ?1", params![jf]);
        }
    }
    HttpResponse::Ok().json(json!({}))
}
