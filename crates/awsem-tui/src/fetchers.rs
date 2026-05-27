use crate::aws_clients::AwsClients;
use crate::types::*;

fn fmt_size(bytes: i64) -> String {
    if bytes < 1024 { format!("{bytes}B") }
    else if bytes < 1024 * 1024 { format!("{:.1}K", bytes as f64 / 1024.0) }
    else if bytes < 1024 * 1024 * 1024 { format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0)) }
    else { format!("{:.1}G", bytes as f64 / (1024.0 * 1024.0 * 1024.0)) }
}

fn fmt0(s: Option<&str>) -> String { s.unwrap_or("").to_string() }

fn fmt_aws_date(d: Option<&aws_sdk_s3::primitives::DateTime>) -> String {
    d.map(|d| {
        use chrono::TimeZone;
        let dt = chrono::Utc.timestamp_opt(d.secs(), d.subsec_nanos()).unwrap();
        dt.format("%m-%d %H:%M").to_string()
    }).unwrap_or_default()
}

pub async fn s3_buckets(aws: &AwsClients) -> Result<Vec<S3Bucket>, String> {
    aws.s3.list_buckets().send().await
        .map(|o| o.buckets().iter().map(|b| S3Bucket {
            name: fmt0(b.name()),
            created: b.creation_date().map(|d| d.to_string()).unwrap_or_default(),
            objects: "—".into(), size: "—".into(),
        }).collect())
        .map_err(|e| format!("S3 ListBuckets failed: {e}"))
}

pub async fn s3_objects(aws: &AwsClients, bucket: &str, prefix: &str) -> Vec<S3Object> {
    aws.s3.list_objects_v2().bucket(bucket).prefix(prefix).delimiter("/").send().await.ok()
        .map(|o| {
            let mut items: Vec<S3Object> = o.common_prefixes().iter().filter_map(|p| p.prefix().map(|s| S3Object {
                key: s.trim_end_matches('/').to_string(),
                size: 0, size_str: "—".into(), modified: "—".into(),
                storage_class: "—".into(), is_folder: true,
            })).collect();
            for obj in o.contents().iter() {
                if let Some(k) = obj.key() {
                    if k == prefix { continue; }
                    items.push(S3Object {
                        key: k.to_string(),
                        size: obj.size().unwrap_or(0),
                        size_str: fmt_size(obj.size().unwrap_or(0)),
                        modified: fmt_aws_date(obj.last_modified()),
                        storage_class: obj.storage_class().map(|s| format!("{s:?}")).unwrap_or_default(),
                        is_folder: false,
                    });
                }
            }
            items
        }).unwrap_or_default()
}

pub async fn cognito_users(aws: &AwsClients) -> Vec<CognitoUser> {
    aws.cognito.list_users().user_pool_id("us-east-1_default").send().await.ok()
        .map(|o| o.users().iter().map(|u| {
            let email = u.attributes().iter().find(|a| a.name() == "email").and_then(|a| a.value()).unwrap_or("").to_string();
            CognitoUser {
                username: fmt0(u.username()),
                status: fmt0(u.user_status().map(|s| format!("{s:?}")).as_deref()),
                email,
                created: u.user_create_date().map(|d| format!("{d}")).unwrap_or_default(),
            }
        }).collect()).unwrap_or_default()
}

pub async fn secrets(aws: &AwsClients) -> Vec<SecretEntry> {
    aws.secrets.list_secrets().send().await.ok()
        .map(|o| o.secret_list().iter().map(|s| SecretEntry {
            name: fmt0(s.name()),
            arn: fmt0(s.arn()),
            description: fmt0(s.description()),
            last_changed: s.last_changed_date().map(|d| {
                use chrono::TimeZone;
                let dt = chrono::Utc.timestamp_opt(d.secs(), d.subsec_nanos()).unwrap();
                dt.format("%m-%d").to_string()
            }).unwrap_or_default(),
            rotation: if s.rotation_enabled().unwrap_or(false) { "ENABLED" } else { "DISABLED" }.into(),
            status: "ACTIVE".into(),
        }).collect()).unwrap_or_default()
}

pub async fn lambda_funcs(aws: &AwsClients) -> Vec<LambdaFn> {
    aws.lambda.list_functions().send().await.ok()
        .map(|o| o.functions().iter().map(|f| LambdaFn {
            name: fmt0(f.function_name()),
            runtime: f.runtime().map(|r| format!("{r:?}")).unwrap_or_default(),
            timeout: f.timeout().unwrap_or(3) as i64,
            handler: fmt0(f.handler()),
            last_modified: fmt0(f.last_modified()),
            memory: f.memory_size().unwrap_or(128),
        }).collect()).unwrap_or_default()
}

pub async fn emr_vcs(aws: &AwsClients) -> Vec<EmrVc> {
    aws.emr.list_virtual_clusters().send().await.ok()
        .map(|o| o.virtual_clusters().iter().map(|v| EmrVc {
            id: fmt0(v.id()),
            name: fmt0(v.name()),
            state: fmt0(v.state().map(|s| format!("{s:?}")).as_deref()),
            namespace: v.container_provider().and_then(|p| p.info()).and_then(|i| i.as_eks_info().ok()).map(|e| e.namespace().unwrap_or("")).unwrap_or("").to_string(),
            jobs: 0,
        }).collect()).unwrap_or_default()
}

pub async fn emr_jobs(aws: &AwsClients, vc: &str) -> Vec<EmrJobRun> {
    aws.emr.list_job_runs().virtual_cluster_id(vc).send().await.ok()
        .map(|o| o.job_runs().iter().map(|j| EmrJobRun {
            id: fmt0(j.id()),
            name: fmt0(j.name()),
            state: format!("{:?}", j.state()),
            exit_code: None,
            logs: None,
            created: j.created_at().map(|d| format!("{d}")).unwrap_or_default(),
        }).collect()).unwrap_or_default()
}

pub async fn s3_all_keys(aws: &AwsClients, bucket: &str, prefix: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut token = None;
    loop {
        let mut req = aws.s3.list_objects_v2().bucket(bucket).prefix(prefix);
        if let Some(t) = token.take() { req = req.continuation_token(t); }
        let Ok(resp) = req.send().await else { break; };
        for obj in resp.contents() {
            if let Some(k) = obj.key() && !k.ends_with('/') { keys.push(k.to_string()); }
        }
        token = resp.next_continuation_token().map(|s| s.to_string());
        if token.is_none() { break; }
    }
    keys
}

pub async fn read_log_file(path: &str, filter: &str) -> Vec<LogEntry> {
    let content = match tokio::fs::read_to_string(path).await { Ok(c) => c, Err(_) => return Vec::new() };
    content.lines().filter_map(|line| {
        if !filter.is_empty() && !line.to_lowercase().contains(&filter.to_lowercase()) { return None; }
        let parts: Vec<&str> = line.split(' ').filter(|s| !s.is_empty()).collect();
        if parts.len() < 2 { return None; }
        let ts = parts[0].to_string();
        let level = parts[1].to_string();
        if !["INFO", "WARN", "ERROR", "DEBUG", "TRACE"].contains(&level.as_str()) { return None; }
        Some(LogEntry { timestamp: ts, level, message: parts[2..].join(" ") })
    }).collect()
}
