use crate::config::RustFsConfig;
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::{PersistentVolumeClaim, Pod, Service};
use kube::api::{Api, PostParams, DeleteParams, ListParams};
use kube::Client;
use serde_json::json;

pub async fn deploy(
    client: &Client,
    cfg: &RustFsConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let ns = &cfg.namespace;

    let pvc_api: Api<PersistentVolumeClaim> = Api::namespaced(client.clone(), ns);
    let pvc: PersistentVolumeClaim = serde_json::from_value(json!({
        "apiVersion": "v1",
        "kind": "PersistentVolumeClaim",
        "metadata": { "name": "rustfs-data", "namespace": ns },
        "spec": {
            "accessModes": ["ReadWriteOnce"],
            "resources": { "requests": { "storage": cfg.pvc_size } }
        }
    }))?;
    let _ = pvc_api.create(&PostParams::default(), &pvc).await;

    let dep_api: Api<Deployment> = Api::namespaced(client.clone(), ns);
    let dep: Deployment = serde_json::from_value(json!({
        "apiVersion": "apps/v1",
        "kind": "Deployment",
        "metadata": { "name": "rustfs", "namespace": ns, "labels": {"app": "rustfs"} },
        "spec": {
            "replicas": 1,
            "selector": { "matchLabels": { "app": "rustfs" } },
            "template": {
                "metadata": { "labels": { "app": "rustfs" } },
                "spec": {
                    "containers": [{
                        "name": "rustfs",
                        "image": cfg.image,
                        "ports": [{"containerPort": 9000}, {"containerPort": 9001}],
                        "env": [
                            {"name": "RUSTFS_ACCESS_KEY", "value": "awsem"},
                            {"name": "RUSTFS_SECRET_KEY", "value": "awsem"},
                        ],
                        "volumeMounts": [{"name": "data", "mountPath": "/data"}]
                    }],
                    "volumes": [{"name": "data", "persistentVolumeClaim": {"claimName": "rustfs-data"}}]
                }
            }
        }
    }))?;
    let _ = dep_api.create(&PostParams::default(), &dep).await;

    let svc_api: Api<Service> = Api::namespaced(client.clone(), ns);
    let svc: Service = serde_json::from_value(json!({
        "apiVersion": "v1",
        "kind": "Service",
        "metadata": { "name": "rustfs-svc", "namespace": ns },
        "spec": {
            "selector": { "app": "rustfs" },
            "ports": [{"port": 9000, "targetPort": 9000, "name": "s3"}]
        }
    }))?;
    let _ = svc_api.create(&PostParams::default(), &svc).await;

    tracing::info!("RustFS deployed to namespace {ns}");
    Ok(())
}

pub async fn wait_ready(client: &Client, ns: &str) -> Result<(), Box<dyn std::error::Error>> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), ns);
    for _ in 0..60 {
        let list = pods.list(&ListParams::default().labels("app=rustfs")).await?;
        for p in list {
            if let Some(status) = p.status
                && status.phase == Some("Running".into()) {
                    return Ok(());
                }
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    Err("timeout waiting for RustFS pod to be Ready".into())
}

pub async fn get_pod_name(
    client: &Client,
    ns: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), ns);
    let list = pods.list(&ListParams::default().labels("app=rustfs")).await?;
    list.into_iter().next()
        .and_then(|p| p.metadata.name)
        .ok_or_else(|| "no RustFS pod found".into())
}

pub async fn cleanup(
    client: &Client,
    cfg: &RustFsConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let ns = &cfg.namespace;
    let dp = DeleteParams::default();

    let dep_api: Api<Deployment> = Api::namespaced(client.clone(), ns);
    let _ = dep_api.delete("rustfs", &dp).await;

    let svc_api: Api<Service> = Api::namespaced(client.clone(), ns);
    let _ = svc_api.delete("rustfs-svc", &dp).await;

    let pvc_api: Api<PersistentVolumeClaim> = Api::namespaced(client.clone(), ns);
    let _ = pvc_api.delete("rustfs-data", &dp).await;

    tracing::info!("RustFS resources cleaned up from namespace {ns}");
    Ok(())
}
