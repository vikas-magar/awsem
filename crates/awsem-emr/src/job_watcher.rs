use awsem_core::db::DbConn;
use awsem_events::BusEvent;
use tokio::sync::broadcast;

struct RunningJob {
    id: String,
    vc_id: String,
    ns: String,
}

pub async fn watch_jobs(
    db: DbConn,
    k8s_client: Option<kube::Client>,
    rustfs_url: String,
    http_client: reqwest::Client,
    event_bus: broadcast::Sender<BusEvent>,
) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
    loop {
        interval.tick().await;
        let jobs = match get_running_jobs(&db) {
            Ok(j) => j,
            Err(e) => {
                tracing::warn!("Failed to fetch running jobs: {e}");
                continue;
            }
        };
        for job in &jobs {
            if let Some(ref client) = k8s_client {
                check_k8s_status(client, &db, &event_bus, job).await;
            }
            check_s3_success(&db, &rustfs_url, &http_client, &event_bus, job).await;
        }
    }
}

async fn check_k8s_status(
    client: &kube::Client,
    db: &DbConn,
    event_bus: &broadcast::Sender<BusEvent>,
    job: &RunningJob,
) {
    let pods: kube::Api<k8s_openapi::api::core::v1::Pod> =
        kube::Api::namespaced(client.clone(), &job.ns);
    let label = format!("awsem-job-run-id={}", job.id);
    let list = match pods.list(&kube::api::ListParams::default().labels(&label)).await {
        Ok(l) => l,
        Err(_) => return,
    };
    for pod in list {
        let phase = pod.status.as_ref().and_then(|s| s.phase.as_deref()).unwrap_or("");
        match phase {
            "Succeeded" => {
                tracing::info!("Job {} driver pod Succeeded", job.id);
                if let Ok(c) = db.lock()
                    && let Err(e) = c.execute(
                        "UPDATE emr_job_runs SET state = 'COMPLETED' WHERE id = ?1",
                        rusqlite::params![job.id],
                    )
                {
                    tracing::error!("Failed to update job state to COMPLETED: {e}");
                }
                if event_bus.send(BusEvent::S3Notification {
                    bucket: format!("emr-{}", job.vc_id),
                    key: format!("jobs/{}/_SUCCESS", job.id),
                    target_arn: format!("arn:aws:emr-containers:us-east-1:000000000000:/virtualclusters/{}/jobruns/{}", job.vc_id, job.id),
                    target_type: "emr".into(),
                }).is_err() {
                    tracing::warn!("No subscribers for S3Notification event");
                }
            }
            "Failed" | "Unknown" => {
                tracing::warn!("Job {} driver pod {}", job.id, phase);
                if let Ok(c) = db.lock()
                    && let Err(e) = c.execute(
                        "UPDATE emr_job_runs SET state = 'FAILED' WHERE id = ?1",
                        rusqlite::params![job.id],
                    )
                {
                    tracing::error!("Failed to update job state to FAILED: {e}");
                }
            }
            _ => {}
        }
    }
}

async fn check_s3_success(
    db: &DbConn,
    rustfs_url: &str,
    http_client: &reqwest::Client,
    _event_bus: &broadcast::Sender<BusEvent>,
    job: &RunningJob,
) {
    if rustfs_url.is_empty() {
        return;
    }
    let url = format!("{}/emr-{}/jobs/{}/_SUCCESS", rustfs_url, job.vc_id, job.id);
    match http_client.head(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            tracing::info!("Job {} _SUCCESS found in S3", job.id);
            if let Ok(c) = db.lock()
                && let Err(e) = c.execute(
                    "UPDATE emr_job_runs SET state = 'COMPLETED' WHERE id = ?1 AND state = 'RUNNING'",
                    rusqlite::params![job.id],
                )
            {
                tracing::error!("Failed to update job state via S3: {e}");
            }
        }
        _ => {}
    }
}

fn get_running_jobs(db: &DbConn) -> Result<Vec<RunningJob>, String> {
    let c = db.lock().map_err(|e| e.to_string())?;
    let mut stmt = c.prepare(
        "SELECT j.id, j.virtual_cluster_id, COALESCE(j.kubernetes_job_name, ''), COALESCE(v.namespace, '')
         FROM emr_job_runs j
         LEFT JOIN emr_virtual_clusters v ON v.id = j.virtual_cluster_id
         WHERE j.state IN ('RUNNING', 'SUBMITTED')"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(RunningJob {
            id: row.get::<_, String>(0)?,
            vc_id: row.get::<_, String>(1)?,
            ns: row.get::<_, String>(3)?,
        })
    }).map_err(|e| e.to_string())?;
    let mut jobs = Vec::new();
    for row in rows {
        jobs.push(row.map_err(|e| e.to_string())?);
    }
    Ok(jobs)
}
