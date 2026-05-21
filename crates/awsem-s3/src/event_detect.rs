use crate::proxy::S3State;
use crate::notification;

pub async fn on_success(state: &S3State, bucket: &str, key: &str) {
    if !key.ends_with("_SUCCESS") {
        return;
    }

    tracing::info!("_SUCCESS detected: s3://{bucket}/{key}");

    let targets = notification::get_event_targets(&state.db, bucket, key);

    for (arn, target_type) in targets {
        tracing::info!(
            "Dispatching _SUCCESS event to {target_type}: {arn}"
        );

        if state.event_bus.receiver_count() > 0
            && state.event_bus.send(
                awsem_events::BusEvent::S3Notification {
                    bucket: bucket.to_string(),
                    key: key.to_string(),
                    target_arn: arn,
                    target_type,
                },
            ).is_err()
        {
            tracing::warn!("No subscribers for S3Notification event");
        }
    }
}

pub async fn check_success_file(
    http_client: &reqwest::Client,
    rustfs_url: &str,
    bucket: &str,
    prefix: &str,
    event_bus: &tokio::sync::broadcast::Sender<awsem_events::BusEvent>,
    db: &awsem_core::db::DbConn,
) {
    let url = format!("{rustfs_url}/{bucket}/{prefix}_SUCCESS");
    match http_client.head(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let state = S3State {
                rustfs_url: rustfs_url.to_string(),
                http_client: http_client.clone(),
                db: db.clone(),
                event_bus: event_bus.clone(),
            };
            let key = format!("{prefix}_SUCCESS");
            on_success(&state, bucket, &key).await;
        }
        Ok(_) => tracing::debug!("_SUCCESS not found at {url}"),
        Err(e) => tracing::warn!("Failed to check _SUCCESS at {url}: {e}"),
    }
}
