use serde_json::{Map, Value};

pub fn build_args(
    job_driver: &Map<String, Value>,
    image: &str,
    namespace: &str,
    job_id: &str,
    rustfs_access_key: &str,
    rustfs_secret_key: &str,
) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    if let Some(driver) = job_driver.get("sparkSubmitJobDriver")
        && let Some(params_str) = driver.get("sparkSubmitParameters").and_then(|v| v.as_str())
    {
        let parts = shlex::split(params_str).unwrap_or_default();
        let mut i = 0;
        while i < parts.len() {
            let p = &parts[i];
            if p == "--master" || p == "--deploy-mode" {
                i += 2;
                continue;
            }
            if p.starts_with("--master=") || p.starts_with("--deploy-mode=") {
                i += 1;
                continue;
            }
            args.push(p.clone());
            i += 1;
        }
    }

    args.push("--master".into());
    args.push("k8s://https://kubernetes.default.svc".into());
    args.push("--deploy-mode".into());
    args.push("cluster".into());
    args.push("--conf".into());
    args.push(format!("spark.kubernetes.container.image={image}"));
    args.push("--conf".into());
    args.push(format!("spark.kubernetes.namespace={namespace}"));
    args.push("--conf".into());
    args.push("spark.kubernetes.authenticate.driver.serviceAccountName=spark".into());
    args.push("--conf".into());
    args.push("spark.kubernetes.authenticate.executor.serviceAccountName=spark".into());
    args.push("--conf".into());
    args.push(format!(
        "spark.kubernetes.driver.label.awsem-job-run-id={job_id}"
    ));
    args.push("--conf".into());
    args.push(format!(
        "spark.kubernetes.executor.label.awsem-job-run-id={job_id}"
    ));

    args.push("--conf".into());
    args.push("spark.hadoop.fs.s3a.endpoint=http://rustfs-svc:9000".into());
    args.push("--conf".into());
    args.push(format!("spark.hadoop.fs.s3a.access.key={rustfs_access_key}"));
    args.push("--conf".into());
    args.push(format!("spark.hadoop.fs.s3a.secret.key={rustfs_secret_key}"));
    args.push("--conf".into());
    args.push("spark.hadoop.fs.s3a.path.style.access=true".into());
    args.push("--conf".into());
    args.push("spark.kubernetes.executor.deleteOnTermination=true".into());
    args.push("--conf".into());
    args.push("spark.kubernetes.driver.deleteOnTermination=false".into());

    if let Some(driver) = job_driver.get("sparkSubmitJobDriver") {
        if let Some(entry) = driver.get("entryPoint").and_then(|v| v.as_str()) {
            args.push(entry.into());
        }
        if let Some(entry_args) = driver.get("entryPointArguments").and_then(|v| v.as_array()) {
            for arg in entry_args {
                if let Some(s) = arg.as_str() {
                    args.push(s.into());
                }
            }
        }
    }

    args
}
