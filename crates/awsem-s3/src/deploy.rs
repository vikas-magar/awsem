use crate::config::RustFsConfig;
use crate::port_forward;
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::{PersistentVolumeClaim, Pod, Service};
use kube::Client;
use kube::api::{Api, DeleteParams, ListParams, PostParams};
use serde_json::json;

pub async fn ensure(client: &Client, cfg: &RustFsConfig) -> Result<String, Box<dyn std::error::Error>> {
    awsem_core::k8s::ensure_namespace(client, &cfg.namespace).await?;
    deploy(client, cfg).await?;
    wait_ready(client, &cfg.namespace).await?;
    let pod_name = get_pod_name(client, &cfg.namespace).await?;
    let port = port_forward::port_forward(client, &cfg.namespace, &pod_name).await?;
    Ok(format!("http://127.0.0.1:{port}"))
}

pub async fn deploy(client: &Client, cfg: &RustFsConfig) -> Result<(), Box<dyn std::error::Error>> {
    let ns = &cfg.namespace;

    let pods: Api<Pod> = Api::namespaced(client.clone(), ns);
    let existing = pods.list(&ListParams::default().labels("app=rustfs")).await?;
    if existing.iter().any(|p| p.status.as_ref().and_then(|s| s.phase.as_deref()) == Some("Running")) {
        tracing::info!("RustFS pod already running, skipping deploy");
        return Ok(());
    }

    let pvc_api: Api<PersistentVolumeClaim> = Api::namespaced(client.clone(), ns);
    match pvc_api.get("rustfs-data").await {
        Ok(_) => tracing::info!("PVC rustfs-data already exists"),
        Err(_) => {
            let pvc: PersistentVolumeClaim = serde_json::from_value(json!({
                "apiVersion": "v1", "kind": "PersistentVolumeClaim",
                "metadata": { "name": "rustfs-data", "namespace": ns },
                "spec": { "accessModes": ["ReadWriteOnce"], "resources": { "requests": { "storage": cfg.pvc_size } } }
            }))?;
            for attempt in 0..10 {
                match pvc_api.create(&PostParams::default(), &pvc).await {
                    Ok(_) => break,
                    Err(kube::Error::Api(e)) if e.code == 409 && attempt < 9 => {
                        let _ = pvc_api.delete("rustfs-data", &DeleteParams::default()).await;
                        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    }
                    Err(kube::Error::Api(e)) if e.code == 409 => return Err(e.into()),
                    Err(e) => return Err(e.into()),
                }
            }
        }
    }

    let dep_api: Api<Deployment> = Api::namespaced(client.clone(), ns);
    if dep_api.get("rustfs").await.is_err() {
        let dep: Deployment = serde_json::from_value(json!({
            "apiVersion": "apps/v1", "kind": "Deployment",
            "metadata": { "name": "rustfs", "namespace": ns, "labels": {"app": "rustfs"} },
            "spec": {
                "replicas": 1, "selector": { "matchLabels": { "app": "rustfs" } },
                "template": {
                    "metadata": { "labels": { "app": "rustfs" } },
                    "spec": {
                        "containers": [{
                            "name": "rustfs", "image": cfg.image,
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
        if let Err(e) = dep_api.create(&PostParams::default(), &dep).await {
            tracing::warn!("Failed to create RustFS Deployment: {e}");
        }
    } else {
        tracing::info!("RustFS Deployment already exists");
    }

    let svc_api: Api<Service> = Api::namespaced(client.clone(), ns);
    if svc_api.get("rustfs-svc").await.is_err() {
        let svc: Service = serde_json::from_value(json!({
            "apiVersion": "v1", "kind": "Service",
            "metadata": { "name": "rustfs-svc", "namespace": ns },
            "spec": { "selector": { "app": "rustfs" }, "ports": [{"port": 9000, "targetPort": 9000, "name": "s3"}] }
        }))?;
        if let Err(e) = svc_api.create(&PostParams::default(), &svc).await {
            tracing::warn!("Failed to create RustFS Service: {e}");
        }
    } else {
        tracing::info!("RustFS Service already exists");
    }

    tracing::info!("RustFS ready in namespace {ns}");
    Ok(())
}

#[rustfmt::skip]
pub async fn wait_ready(client: &Client, ns: &str) -> Result<(), Box<dyn std::error::Error>> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), ns);
    for _ in 0..60 {
        if pods.list(&ListParams::default().labels("app=rustfs")).await?.into_iter().any(|p| p.status.and_then(|s| s.phase).as_deref() == Some("Running")) { return Ok(()); }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    Err("timeout waiting for RustFS pod to be Ready".into())
}

#[rustfmt::skip]
pub async fn get_pod_name(client: &Client, ns: &str) -> Result<String, Box<dyn std::error::Error>> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), ns);
    pods.list(&ListParams::default().labels("app=rustfs")).await?.into_iter().next().and_then(|p| p.metadata.name).ok_or_else(|| "no RustFS pod found".into())
}

pub async fn cleanup(client: &Client, cfg: &RustFsConfig) -> Result<(), Box<dyn std::error::Error>> {
    let ns = &cfg.namespace;
    let dp = DeleteParams::default();
    let dep_api: Api<Deployment> = Api::namespaced(client.clone(), ns);
    if let Err(e) = dep_api.delete("rustfs", &dp).await { tracing::warn!("Failed to delete Deployment: {e}"); }
    let svc_api: Api<Service> = Api::namespaced(client.clone(), ns);
    if let Err(e) = svc_api.delete("rustfs-svc", &dp).await { tracing::warn!("Failed to delete Service: {e}"); }
    let pvc_api: Api<PersistentVolumeClaim> = Api::namespaced(client.clone(), ns);
    if let Err(e) = pvc_api.delete("rustfs-data", &dp).await { tracing::warn!("Failed to delete PVC: {e}"); }
    tracing::info!("RustFS resources cleaned up from namespace {ns}");
    Ok(())
}
