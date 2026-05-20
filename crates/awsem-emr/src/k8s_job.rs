use crate::spark_params;
use crate::AppState;
use actix_web::{web, HttpResponse};
use rusqlite::params;
use serde_json::{json, Value};
use kube::api::PostParams;
use k8s_openapi::api::batch::v1::Job;
use k8s_openapi::api::core::v1::ServiceAccount;
use k8s_openapi::api::rbac::v1::{Role, RoleBinding};

fn get_vc_info(state: &AppState, vc_id: &str) -> Option<(String, String)> {
    let c = state.db.lock().ok()?;
    c.query_row(
        "SELECT namespace, name FROM emr_virtual_clusters WHERE id = ?1",
        params![vc_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    ).ok()
}

pub async fn start_job_run(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: bytes::Bytes,
) -> HttpResponse {
    let vc_id = path.into_inner();
    let input: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return awsem_core::error::AwsemError::InvalidRequest(e.to_string()).to_response(),
    };
    let name = input.get("name").and_then(|v| v.as_str()).unwrap_or("emr-job");
    let release = input.get("releaseLabel").and_then(|v| v.as_str()).unwrap_or("emr-7.1.0-latest");
    let job_driver = input.get("jobDriver").and_then(|v| v.as_object()).cloned().unwrap_or_default();
    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:emr-containers:us-east-1:000000000000:/virtualclusters/{vc_id}/jobruns/{id}");
    let job_driver_json = serde_json::to_string(&job_driver).unwrap_or_default();
    let job_name = format!("emr-job-{id}");
    {
        let c = match state.db.lock() {
            Ok(c) => c,
            Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
        };
        if let Err(e) = c.execute(
            "INSERT INTO emr_job_runs (id, name, arn, virtual_cluster_id, job_driver_json, state, kubernetes_job_name) VALUES (?1, ?2, ?3, ?4, ?5, 'SUBMITTED', ?6)",
            params![id, name, arn, vc_id, job_driver_json, job_name],
        ) {
            return awsem_core::error::AwsemError::AlreadyExists(e.to_string()).to_response();
        }
    }
    if let (Some(client), Some((vc_ns, _))) = (&state.k8s_client, get_vc_info(&state, &vc_id)) {
        ensure_rbac(client, &vc_ns).await;
        let _ = submit_k8s_job(client, &vc_ns, &id, &job_name, release, &job_driver).await;
    }
    let _ = state.db.lock().map(|c| c.execute(
        "UPDATE emr_job_runs SET state = 'RUNNING' WHERE id = ?1",
        params![id],
    ));
    HttpResponse::Ok().json(json!({
        "id": id, "arn": arn, "name": name, "state": "RUNNING",
        "virtualClusterId": vc_id,
    }))
}

async fn ensure_rbac(client: &kube::Client, namespace: &str) {
    let sa_api: kube::Api<ServiceAccount> = kube::Api::namespaced(client.clone(), namespace);
    let sa: ServiceAccount = serde_json::from_value(json!({
        "apiVersion": "v1",
        "kind": "ServiceAccount",
        "metadata": { "name": "spark", "namespace": namespace }
    })).unwrap();
    let _ = sa_api.create(&PostParams::default(), &sa).await;

    let role_api: kube::Api<Role> = kube::Api::namespaced(client.clone(), namespace);
    let role: Role = serde_json::from_value(json!({
        "apiVersion": "rbac.authorization.k8s.io/v1",
        "kind": "Role",
        "metadata": { "name": "spark-role", "namespace": namespace },
        "rules": [{
            "apiGroups": [""],
            "resources": ["pods", "pods/log", "pods/status", "services", "configmaps", "persistentvolumeclaims"],
            "verbs": ["*"]
        }]
    })).unwrap();
    let _ = role_api.create(&PostParams::default(), &role).await;

    let rb_api: kube::Api<RoleBinding> = kube::Api::namespaced(client.clone(), namespace);
    let rb: RoleBinding = serde_json::from_value(json!({
        "apiVersion": "rbac.authorization.k8s.io/v1",
        "kind": "RoleBinding",
        "metadata": { "name": "spark-binding", "namespace": namespace },
        "subjects": [{ "kind": "ServiceAccount", "name": "spark", "namespace": namespace }],
        "roleRef": { "kind": "Role", "name": "spark-role", "apiGroup": "rbac.authorization.k8s.io" }
    })).unwrap();
    let _ = rb_api.create(&PostParams::default(), &rb).await;
}

async fn submit_k8s_job(
    client: &kube::Client,
    namespace: &str,
    job_id: &str,
    job_name: &str,
    release: &str,
    job_driver: &serde_json::Map<String, Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    let image = if release.starts_with("emr-") {
        "apache/spark:latest".into()
    } else {
        format!("public.ecr.aws/emr-on-eks/spark/{release}")
    };
    let spark_args = spark_params::build_args(job_driver, &image, namespace, job_id);
    let job_obj: Job = serde_json::from_value(json!({
        "apiVersion": "batch/v1",
        "kind": "Job",
        "metadata": { "name": job_name, "namespace": namespace },
        "spec": {
            "template": {
                "spec": {
                    "serviceAccountName": "spark",
                    "restartPolicy": "Never",
                    "containers": [{
                        "name": "spark-driver",
                        "image": image,
                        "command": ["/opt/spark/bin/spark-submit"],
                        "args": spark_args,
                    }]
                }
            }
        }
    }))?;
    let api: kube::Api<Job> = kube::Api::namespaced(client.clone(), namespace);
    api.create(&PostParams::default(), &job_obj).await?;
    tracing::info!("Created K8s Job {job_name} in namespace {namespace}");
    Ok(())
}
