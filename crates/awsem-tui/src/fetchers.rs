use crate::aws_clients::AwsClients;

fn fmt0(s: Option<&str>) -> String { s.unwrap_or("").to_string() }

pub async fn s3_buckets(aws: &AwsClients) -> Vec<String> {
    aws.s3.list_buckets().send().await.ok()
        .map(|o| o.buckets().iter().map(|b| fmt0(b.name())).collect())
        .unwrap_or_default()
}

pub async fn s3_folder_objects(aws: &AwsClients, bucket: &str, prefix: &str) -> (Vec<String>, Vec<(String, i64, String)>) {
    aws.s3.list_objects_v2().bucket(bucket).prefix(prefix).delimiter("/").send().await.ok()
        .map(|o| {
            let folders: Vec<String> = o.common_prefixes().iter().filter_map(|p| p.prefix().map(String::from)).collect();
            let files: Vec<(String, i64, String)> = o.contents().iter().filter_map(|obj| {
                let k = obj.key()?;
                if k == prefix { return None; }
                let m = obj.last_modified().map(|d| d.to_string()).unwrap_or_default();
                let short = if m.len() > 16 { m[..16].to_string() } else { m };
                Some((k.to_string(), obj.size().unwrap_or(0), short))
            }).collect();
            (folders, files)
        }).unwrap_or_default()
}

pub async fn cognito_users(aws: &AwsClients) -> Vec<(String, String, String)> {
    aws.cognito.list_users().user_pool_id("us-east-1_default").send().await.ok()
        .map(|o| o.users().iter().map(|u| {
            let email = u.attributes().iter()
                .find(|a| a.name() == "email").and_then(|a| a.value()).unwrap_or("").to_string();
            (fmt0(u.username()), fmt0(u.user_status().map(|s| format!("{s:?}")).as_deref()), email)
        }).collect()).unwrap_or_default()
}

pub async fn secrets(aws: &AwsClients) -> Vec<(String, String, String)> {
    aws.secrets.list_secrets().send().await.ok()
        .map(|o| o.secret_list().iter().map(|s| {
            (fmt0(s.name()), fmt0(s.description()), s.last_changed_date().map(|d| format!("{d}")).unwrap_or_default())
        }).collect()).unwrap_or_default()
}

pub async fn lambda_funcs(aws: &AwsClients) -> Vec<(String, String, i64)> {
    aws.lambda.list_functions().send().await.ok()
        .map(|o| o.functions().iter().map(|f| {
            (fmt0(f.function_name()), fmt0(f.runtime().map(|r| format!("{r:?}")).as_deref()), f.timeout().unwrap_or(3) as i64)
        }).collect()).unwrap_or_default()
}

pub async fn emr_vcs(aws: &AwsClients) -> Vec<(String, String, String)> {
    aws.emr.list_virtual_clusters().send().await.ok()
        .map(|o| o.virtual_clusters().iter().map(|v| {
            (fmt0(v.id()), fmt0(v.name()), fmt0(v.state().map(|s| format!("{s:?}")).as_deref()))
        }).collect()).unwrap_or_default()
}

pub async fn emr_jobs(aws: &AwsClients, vc: &str) -> Vec<(String, String, String)> {
    aws.emr.list_job_runs().virtual_cluster_id(vc).send().await.ok()
        .map(|o| o.job_runs().iter().map(|j| {
            (fmt0(j.id()), fmt0(j.name()), format!("{:?}", j.state()))
        }).collect()).unwrap_or_default()
}

pub async fn logs(aws: &AwsClients) -> Vec<(String, String, String, String)> {
    // Logs are not an AWS service — returned empty for Overview fetches
    let _ = aws;
    Vec::new()
}

pub async fn read_log_file(path: &str, filter: &str) -> Vec<(String, String, String, String)> {
    let content = match tokio::fs::read_to_string(path).await {
        Ok(c) => c, Err(_) => return Vec::new(),
    };
    content.lines().filter_map(|line| {
        if !filter.is_empty() && !line.to_lowercase().contains(&filter.to_lowercase()) { return None; }
        let parts: Vec<&str> = line.split(' ').filter(|s| !s.is_empty()).collect();
        if parts.len() < 2 { return None; }
        let ts = parts[0].to_string();
        let level = parts[1].to_string();
        if !["INFO", "WARN", "ERROR", "DEBUG", "TRACE"].contains(&level.as_str()) { return None; }
        let msg = parts[2..].join(" ");
        Some((ts, level, String::new(), msg))
    }).collect()
}
