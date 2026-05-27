use crate::spawner;
use crate::AppState;
use actix_web::{HttpResponse, web};
use rusqlite::params;
use serde_json::{Value, json};

fn sid(jr_id: &str) -> String { format!("s-{}", &jr_id[..8]) }

pub async fn submit_step(state: &AppState, vc_id: &str, step: &Value, id: &str) {
    let step_id = sid(id);
    let sname = step.get("Name").and_then(|v| v.as_str()).unwrap_or("step");
    let jar = step.pointer("/HadoopJarStep/Jar").and_then(|v| v.as_str()).unwrap_or("local:///opt/spark/examples/src/main/python/pi.py");
    let args: Vec<String> = step.pointer("/HadoopJarStep/Args").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    let jd = json!({"sparkSubmitJobDriver": {"entryPoint": jar, "entryPointArguments": args, "sparkSubmitParameters": ""}});
    let job_name = format!("emr-step-{id}");
    {
        let c = match state.db.lock() { Ok(c) => c, Err(_) => return };
        let arn = state.aws.emr_job_arn(vc_id, id);
        let _ = c.execute("INSERT INTO emr_job_runs (id, name, arn, virtual_cluster_id, job_driver_json, state, kubernetes_job_name, step_id) VALUES (?1, ?2, ?3, ?4, ?5, 'SUBMITTED', ?6, ?7)", params![id, sname, arn, vc_id, jd.to_string(), job_name, step_id]);
    }
    if let (Some(client), Some(ns)) = (&state.k8s_client, {
        let c = match state.db.lock() { Ok(c) => c, Err(_) => return };
        c.query_row("SELECT namespace FROM emr_virtual_clusters WHERE id = ?1", params![vc_id], |r| r.get::<_,String>(0)).ok()
    }) {
        spawner::ensure_rbac(client, &ns).await;
        let job_driver = jd.as_object().cloned().unwrap_or_default();
        let _ = spawner::submit_k8s_job(client, &ns, id, &job_name, &state.emr_spark_image, &job_driver, &state.awsem_endpoint).await;
    }
}

pub async fn add_job_flow_steps(state: web::Data<AppState>, body: bytes::Bytes) -> HttpResponse {
    let input: Value = serde_json::from_slice(&body).unwrap_or_default();
    let jf = input.get("JobFlowId").and_then(|v| v.as_str()).unwrap_or("");
    let vc_id = match state.db.lock().ok().and_then(|c| c.query_row("SELECT virtual_cluster_id FROM emr_classic_clusters WHERE job_flow_id = ?1", params![jf], |r| r.get::<_,String>(0)).ok()) {
        Some(v) => v, None => return awsem_core::error::AwsemError::NotFound(jf.into()).to_response(),
    };
    let steps = input.get("Steps").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut ids = Vec::new();
    for step in &steps {
        let id = uuid::Uuid::new_v4().to_string();
        let step_id = sid(&id);
        ids.push(step_id);
        submit_step(&state, &vc_id, step, &id).await;
    }
    HttpResponse::Ok().json(json!({"StepIds": ids}))
}

pub async fn list_steps(state: web::Data<AppState>, body: bytes::Bytes) -> HttpResponse {
    let input: Value = serde_json::from_slice(&body).unwrap_or_default();
    let jf = input.get("ClusterId").and_then(|v| v.as_str()).unwrap_or("");
    let c = awsem_core::lock_db!(state);
    let vc_id = match c.query_row("SELECT virtual_cluster_id FROM emr_classic_clusters WHERE job_flow_id = ?1", params![jf], |r| r.get::<_,String>(0)) {
        Ok(v) => v, Err(_) => return awsem_core::error::AwsemError::NotFound(jf.into()).to_response(),
    };
    let mut stmt = match c.prepare("SELECT id, name, state, created_at, COALESCE(step_id,'') FROM emr_job_runs WHERE virtual_cluster_id = ?1 ORDER BY created_at ASC") {
        Ok(s) => s, Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
    let rows = match stmt.query_map(params![vc_id], |row| {
        let jr_id = row.get::<_,String>(0)?;
        let st = row.get::<_,String>(4)?;
        let sid = if st.is_empty() { format!("s-{}", &jr_id[..8]) } else { st };
        Ok(json!({"Id": sid, "Name": row.get::<_,String>(1)?, "Status": {"State": row.get::<_,String>(2)?, "Timeline": {"CreationDateTime": crate::emr_classic::to_rfc3339(&row.get::<_,String>(3)?)}}}))
    }) { Ok(r) => r, Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response() };
    let mut steps = Vec::new();
    for row in rows { match row { Ok(v) => steps.push(v), Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response() } }
    HttpResponse::Ok().json(json!({"Steps": steps}))
}

pub async fn describe_step(state: web::Data<AppState>, body: bytes::Bytes) -> HttpResponse {
    let input: Value = serde_json::from_slice(&body).unwrap_or_default();
    let jf = input.get("ClusterId").and_then(|v| v.as_str()).unwrap_or("");
    let sid_raw = input.get("StepId").and_then(|v| v.as_str()).unwrap_or("");
    let c = awsem_core::lock_db!(state);
    let vc_id = match c.query_row("SELECT virtual_cluster_id FROM emr_classic_clusters WHERE job_flow_id = ?1", params![jf], |r| r.get::<_,String>(0)) {
        Ok(v) => v, Err(_) => return awsem_core::error::AwsemError::NotFound(jf.into()).to_response(),
    };
    let pattern = if sid_raw.len() > 2 { format!("{}%", &sid_raw[2..]) } else { String::new() };
    match c.query_row(
        "SELECT name, state, created_at, COALESCE(logs,''), COALESCE(exit_code,0) FROM emr_job_runs WHERE virtual_cluster_id = ?1 AND id LIKE ?2",
        params![vc_id, pattern],
        |row| Ok(json!({"Id": sid_raw, "Name": row.get::<_,String>(0)?, "Status": {"State": row.get::<_,String>(1)?, "Timeline": {"CreationDateTime": crate::emr_classic::to_rfc3339(&row.get::<_,String>(2)?)}}, "Logs": row.get::<_,String>(3)?, "ExitCode": row.get::<_,i64>(4)?})),
    ) { Ok(s) => HttpResponse::Ok().json(json!({"Step": s})), Err(_) => awsem_core::error::AwsemError::NotFound(sid_raw.into()).to_response() }
}
