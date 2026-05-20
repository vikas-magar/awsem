use awsem_core::db::DbConn;

pub fn put_notification_config(
    conn: &DbConn,
    bucket: &str,
    config_xml: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let lock = conn.lock().unwrap();
    lock.execute(
        "INSERT OR REPLACE INTO s3_notification_configs (bucket_name, config_xml) VALUES (?1, ?2)",
        [bucket, config_xml],
    )?;
    Ok(())
}

pub fn get_notification_config(
    conn: &DbConn,
    bucket: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let lock = conn.lock().unwrap();
    let mut stmt = lock.prepare(
        "SELECT config_xml FROM s3_notification_configs WHERE bucket_name = ?1",
    )?;
    let mut rows = stmt.query([bucket])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn get_event_targets(
    conn: &DbConn,
    bucket: &str,
    _object_key: &str,
) -> Vec<(String, String)> {
    let config = match get_notification_config(conn, bucket) {
        Ok(Some(c)) => c,
        _ => return vec![],
    };

    let mut targets = Vec::new();
    // Simple parsing: look for QueueArn / TopicArn / CloudFunctionArn entries
    for line in config.lines() {
        let trimmed = line.trim();
        if let Some(arn) = trimmed.strip_prefix("<QueueArn>")
            && let Some(end) = arn.find("</QueueArn>") {
                targets.push((arn[..end].to_string(), "SQS".into()));
            }
        if let Some(arn) = trimmed.strip_prefix("<TopicArn>")
            && let Some(end) = arn.find("</TopicArn>") {
                targets.push((arn[..end].to_string(), "SNS".into()));
            }
        if let Some(arn) = trimmed.strip_prefix("<CloudFunctionArn>")
            && let Some(end) = arn.find("</CloudFunctionArn>") {
                targets.push((arn[..end].to_string(), "Lambda".into()));
            }
    }
    targets
}
