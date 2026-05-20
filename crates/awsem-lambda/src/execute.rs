use crate::AppState;
use k8s_openapi::api::batch::v1::Job;
use kube::api::PostParams;
use serde_json::json;
use uuid::Uuid;

pub async fn run(
    name: &str,
    payload: &str,
    state: &AppState,
) -> String {
    let client = match &state.kube_client {
        Some(c) => c.clone(),
        None => {
            tracing::warn!("No K8s client, returning mock response for Lambda {name}");
            return json!({"statusCode": 200, "body": format!("Hello from awsem Lambda ({name})!")}).to_string();
        }
    };

    let func = {
        let c = match state.db.lock() {
            Ok(c) => c,
            Err(e) => return json!({"statusCode": 500, "body": format!("DB error: {e}")}).to_string(),
        };
        match c.query_row(
            "SELECT handler, image, timeout FROM lambda_functions WHERE name = ?1",
            rusqlite::params![name],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, i32>(2)?)),
        ) {
            Ok(f) => f,
            Err(_) => return json!({"statusCode": 404, "body": format!("Function {name} not found")}).to_string(),
        }
    };

    let image = match &func.1 {
        Some(img) => img.clone(),
        None => match &state.lambda_runtime_image {
            Some(img) => img.clone(),
            None => {
                return json!({"statusCode": 500, "body": String::from("No Lambda runtime image configured")}).to_string();
            }
        },
    };

    let job_id = Uuid::new_v4().to_string();
    let job_name = format!("lambda-{name}-{job_id}");
    let namespace = &state.namespace;

    let job_obj: Job = serde_json::from_value(json!({
        "apiVersion": "batch/v1",
        "kind": "Job",
        "metadata": {
            "name": job_name,
            "namespace": namespace,
        },
        "spec": {
            "backoffLimit": 0,
            "ttlSecondsAfterFinished": 60,
            "template": {
                "spec": {
                    "restartPolicy": "Never",
                    "containers": [{
                        "name": "lambda-runner",
                        "image": image,
                        "imagePullPolicy": "IfNotPresent",
                        "env": [
                            {"name": "_HANDLER", "value": func.0},
                            {"name": "_PAYLOAD", "value": payload},
                        ]
                    }]
                }
            }
        }
    }))
    .expect("static JSON should be valid");

    let api: kube::Api<Job> = kube::Api::namespaced(client.clone(), namespace);
    if let Err(e) = api.create(&PostParams::default(), &job_obj).await {
        return json!({"statusCode": 500, "body": format!("Failed to create K8s Job: {e}")}).to_string();
    }
    tracing::info!("Created K8s Job {job_name} for Lambda {name}");

    let timeout_secs = func.2.max(3) as u64 + 10;
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(timeout_secs);

    let pod_api: kube::Api<k8s_openapi::api::core::v1::Pod> = kube::Api::namespaced(client.clone(), namespace);

    loop {
        if tokio::time::Instant::now() >= deadline {
            let _ = api.delete(&job_name, &Default::default()).await;
            return json!({"statusCode": 500, "body": String::from("Lambda timed out")}).to_string();
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let pods = pod_api
            .list(&kube::api::ListParams::default().labels(&format!("job-name={job_name}")))
            .await;

        let pods = match pods {
            Ok(p) => p,
            Err(_) => continue,
        };

        let pod = match pods.items.into_iter().next() {
            Some(p) => p,
            None => continue,
        };

        let phase = pod
            .status
            .as_ref()
            .and_then(|s| s.phase.as_deref())
            .unwrap_or("");

        if phase == "Succeeded" || phase == "Failed" {
            let log_result = pod_api
                .logs(pod.metadata.name.as_deref().unwrap_or(""), &Default::default())
                .await;

            let _ = api.delete(&job_name, &Default::default()).await;

            return match log_result {
                Ok(logs) => {
                    if phase == "Succeeded" {
                        logs.trim().to_string()
                    } else {
                        json!({"statusCode": 500, "body": logs}).to_string()
                    }
                }
                Err(e) => json!({"statusCode": 500, "body": format!("Log read error: {e}")}).to_string(),
            };
        }
    }
}
