use crate::spark_params;
use k8s_openapi::api::{
    batch::v1::Job,
    core::v1::ServiceAccount,
    rbac::v1::{Role, RoleBinding},
};
use kube::api::PostParams;
use serde_json::{Value, json};

pub async fn ensure_rbac(client: &kube::Client, namespace: &str) {
    let sa_api: kube::Api<ServiceAccount> = kube::Api::namespaced(client.clone(), namespace);
    let sa: ServiceAccount = serde_json::from_value(json!({
        "apiVersion": "v1",
        "kind": "ServiceAccount",
        "metadata": { "name": "spark", "namespace": namespace }
    }))
    .unwrap();
    if let Err(e) = sa_api.create(&PostParams::default(), &sa).await {
        tracing::warn!("Failed to create Spark ServiceAccount: {e}");
    }

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
    if let Err(e) = role_api.create(&PostParams::default(), &role).await {
        tracing::warn!("Failed to create Spark Role: {e}");
    }

    let rb_api: kube::Api<RoleBinding> = kube::Api::namespaced(client.clone(), namespace);
    let rb: RoleBinding = serde_json::from_value(json!({
        "apiVersion": "rbac.authorization.k8s.io/v1",
        "kind": "RoleBinding",
        "metadata": { "name": "spark-binding", "namespace": namespace },
        "subjects": [{ "kind": "ServiceAccount", "name": "spark", "namespace": namespace }],
        "roleRef": { "kind": "Role", "name": "spark-role", "apiGroup": "rbac.authorization.k8s.io" }
    }))
    .unwrap();
    if let Err(e) = rb_api.create(&PostParams::default(), &rb).await {
        tracing::warn!("Failed to create Spark RoleBinding: {e}");
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn submit_k8s_job(
    client: &kube::Client,
    namespace: &str,
    job_id: &str,
    job_name: &str,
    release: &str,
    emr_spark_image: Option<&str>,
    job_driver: &serde_json::Map<String, Value>,
    awsem_endpoint: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let image = if release.starts_with("emr-") {
        emr_spark_image.unwrap_or("apache/spark:latest").to_string()
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
                        "imagePullPolicy": "IfNotPresent",
                        "resources": {"requests": {"cpu": "500m", "memory": "512Mi"}, "limits": {"cpu": "1", "memory": "1Gi"}},
                        "command": ["/opt/spark/bin/awsem-spark-agent"],
                        "args": spark_args,
                        "env": [
                            {"name": "JOB_RUN_ID", "value": job_id},
                            {"name": "AWSEM_ENDPOINT", "value": awsem_endpoint},
                        ],
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
