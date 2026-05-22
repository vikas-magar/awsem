use crate::AppState;
use futures::StreamExt;
use k8s_openapi::api::batch::v1::Job;
use kube::api::PostParams;
use kube::runtime::WatchStreamExt;
use kube::runtime::watcher;
use serde_json::json;
use uuid::Uuid;

#[tracing::instrument(skip(state), fields(name = %name))]
pub async fn run(name: &str, payload: &str, state: &AppState) -> String {
    let client = match &state.kube_client {
        Some(c) => c.clone(),
        None => {
            tracing::warn!("No K8s client, returning mock response for Lambda {name}");
            return json!({"statusCode": 200, "body": format!("Hello from awsem Lambda ({name})!")}).to_string();
        }
    };

    let (handler, image, timeout) = {
        let c = match state.db.lock() {
            Ok(c) => c,
            Err(e) => {
                return json!({"statusCode": 500, "body": format!("DB error: {e}")}).to_string();
            }
        };
        match c.query_row(
            "SELECT handler, image, timeout FROM lambda_functions WHERE name = ?1",
            rusqlite::params![name],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, i32>(2)?,
                ))
            },
        ) {
            Ok(f) => f,
            Err(_) => {
                return json!({"statusCode": 404, "body": format!("Function {name} not found")})
                    .to_string();
            }
        }
    };

    let image = match image.or_else(|| state.lambda_runtime_image.clone()) {
        Some(i) => i,
        None => return json!({"statusCode": 500, "body": String::from("No Lambda runtime image configured")}).to_string(),
    };

    let job_id = Uuid::new_v4().to_string();
    let job_name = format!("lambda-{name}-{job_id}");
    let secret_name = format!("lambda-payload-{job_id}");
    let namespace = &state.namespace;

    if let Err(e) =
        crate::extract::create_secret(&client, namespace, &secret_name, "input", payload).await
    {
        return json!({"statusCode": 500, "body": format!("Failed to create payload Secret: {e}")})
            .to_string();
    }

    let job_obj: Job = serde_json::from_value(json!({
        "apiVersion": "batch/v1",
        "kind": "Job",
        "metadata": { "name": job_name, "namespace": namespace },
        "spec": {
            "backoffLimit": 0,
            "ttlSecondsAfterFinished": 60,
            "template": {
                "spec": {
                    "restartPolicy": "Never",
                    "volumes": [{
                        "name": "payload",
                        "secret": { "secretName": secret_name, "items": [{"key": "input", "path": "input"}] }
                    }],
                    "containers": [{
                        "name": "lambda-runner",
                        "image": image,
                        "imagePullPolicy": "IfNotPresent",
                        "resources": {"requests": {"cpu": "250m", "memory": "256Mi"}, "limits": {"cpu": "500m", "memory": "512Mi"}},
                        "volumeMounts": [{"name": "payload", "mountPath": "/var/payload", "readOnly": true}],
                        "env": [
                            {"name": "_HANDLER", "value": handler},
                            {"name": "_PAYLOAD_FILE", "value": "/var/payload/input"},
                        ]
                    }]
                }
            }
        }
    }))
    .expect("static JSON should be valid");

    let job_api: kube::Api<Job> = kube::Api::namespaced(client.clone(), namespace);
    if let Err(e) = job_api.create(&PostParams::default(), &job_obj).await {
        crate::extract::delete_secret(&client, namespace, &secret_name).await;
        return json!({"statusCode": 500, "body": format!("Failed to create K8s Job: {e}")})
            .to_string();
    }
    tracing::info!(job_name, "Created K8s Job for Lambda");

    let pod_api: kube::Api<k8s_openapi::api::core::v1::Pod> =
        kube::Api::namespaced(client.clone(), namespace);

    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(timeout.max(3) as u64 + 10);
    let mut stream = watcher(pod_api.clone(), watcher::Config::default().labels(&format!("job-name={job_name}")))
        .applied_objects().boxed();

    loop {
        tokio::select! {
            Some(item) = stream.next() => {
                let pod = match item {
                    Ok(p) => p,
                    Err(e) => { tracing::warn!("Watch error: {e}"); continue; }
                };
                let phase = pod.status.as_ref().and_then(|s| s.phase.as_deref()).unwrap_or("");
                if phase == "Succeeded" || phase == "Failed" {
                    let pod_name = pod.metadata.name.as_deref().unwrap_or("");
                    let log_result = pod_api.logs(pod_name, &Default::default()).await;
                    job_api.delete(&job_name, &Default::default()).await.ok();
                    crate::extract::delete_secret(&client, namespace, &secret_name).await;
                    return match log_result {
                        Ok(logs) => if phase == "Succeeded" { logs.trim().to_string() }
                            else { json!({"statusCode": 500, "body": logs}).to_string() },
                        Err(e) => json!({"statusCode": 500, "body": format!("Log read error: {e}")}).to_string(),
                    };
                }
            }
            _ = tokio::time::sleep_until(deadline) => {
                job_api.delete(&job_name, &Default::default()).await.ok();
                crate::extract::delete_secret(&client, namespace, &secret_name).await;
                return json!({"statusCode": 500, "body": String::from("Lambda timed out")}).to_string();
            }
        }
    }
}
