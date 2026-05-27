use crate::AppState;
use serde_json::json;
use tokio::sync::broadcast;

fn build_s3_payload(bucket: &str, key: &str) -> String {
    json!({
        "Records": [{
            "eventSource": "aws:s3",
            "eventName": "ObjectCreated:Put",
            "s3": {
                "bucket": {"name": bucket},
                "object": {"key": key},
            },
        }]
    })
    .to_string()
}

pub async fn listen(state: AppState) {
    let mut rx = state.event_bus.subscribe();
    loop {
        match rx.recv().await {
            Ok(awsem_events::BusEvent::S3Notification {
                bucket,
                key,
                target_arn,
                target_type,
            }) => {
                let payload = build_s3_payload(&bucket, &key);

                if target_type == "Lambda" {
                    let name = {
                        let c = match state.db.lock() {
                            Ok(c) => c,
                            Err(_) => continue,
                        };
                        c.query_row(
                            "SELECT name FROM lambda_functions WHERE arn = ?1",
                            rusqlite::params![target_arn],
                            |row| row.get::<_, String>(0),
                        )
                    };
                    if let Ok(name) = name {
                        tracing::info!("Triggering Lambda {name} from S3 notification");
                        crate::execute::run(&name, &payload, &state).await;
                    }
                }

                let rows = {
                    let c = match state.db.lock() {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    let event_arn = state.aws.s3_bucket_arn(&bucket);
                    let mut stmt = match c.prepare(
                        "SELECT f.name
                         FROM lambda_event_source_mappings m
                         JOIN lambda_functions f ON m.function_arn = f.arn
                         WHERE m.event_source_arn = ?1 AND m.enabled = 1",
                    ) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    let rows: Vec<String> = stmt
                        .query_map(rusqlite::params![event_arn], |row| row.get::<_, String>(0))
                        .into_iter()
                        .flatten()
                        .flatten()
                        .collect();
                    rows
                };
                for name in rows {
                    tracing::info!(
                        "Triggering Lambda {name} from event source mapping (S3:{bucket}/{key})"
                    );
                    crate::execute::run(&name, &payload, &state).await;
                }
            }
            Ok(awsem_events::BusEvent::LambdaInvocation { .. }) => {}
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
}
