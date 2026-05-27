use crate::config::{AppConfig, ConfigFile};

fn awsem_home() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(std::path::PathBuf::from(home).join(".config").join("awsem"))
}

pub fn default_config_path() -> Option<String> {
    Some(awsem_home()?.join("config.toml").to_string_lossy().into())
}

pub fn default_data_dir() -> Option<String> {
    Some(awsem_home()?.join("data").to_string_lossy().into())
}

pub fn default_log_dir() -> Option<String> {
    Some(awsem_home()?.join("logs").to_string_lossy().into())
}

pub fn default_db_path(port: u16) -> String {
    awsem_home().map(|p| p.join("awsem.db").to_string_lossy().into()).unwrap_or_else(|| format!("awsem-{port}.db"))
}

pub fn default_jwt_secret() -> String { uuid::Uuid::new_v4().to_string() }

impl AppConfig {
    pub fn merge(cli: Self, file: ConfigFile) -> Self {
        let port = cli.port.or(file.port).unwrap_or(4566);
        let db_path = cli.db_path.or(file.db_path).unwrap_or_else(|| default_db_path(port));
        let k8s_namespace = cli.k8s_namespace.or(file.k8s_namespace).unwrap_or_else(|| "awsem".into());
        let rustfs_pvc_size = cli.rustfs_pvc_size.or(file.rustfs_pvc_size).unwrap_or_else(|| "10Gi".into());
        let no_s3 = cli.no_s3.or(file.no_s3).unwrap_or(false);
        let no_emr = cli.no_emr.or(file.no_emr).unwrap_or(false);
        let no_lambda = cli.no_lambda.or(file.no_lambda).unwrap_or(false);
        let data_dir = cli.data_dir.or(file.data_dir).or_else(default_data_dir);
        let log_dir = cli.log_dir.or(file.log_dir).or_else(default_log_dir);
        let log_level = cli.log_level.or(file.log_level).unwrap_or_else(|| "info".into());
        let jwt_secret = cli.jwt_secret.clone().or(file.jwt_secret).unwrap_or_else(default_jwt_secret);
        let rustfs_image = cli.rustfs_image.or(file.rustfs_image).unwrap_or_else(|| "rustfs/rustfs:latest".into());
        let emr_spark_image = cli.emr_spark_image.or(file.emr_spark_image).unwrap_or_else(|| "spark-s3a:latest".into());
        let region = cli.region.or(file.region).unwrap_or_else(|| "us-east-1".into());
        let account_id = cli.account_id.or(file.account_id).unwrap_or_else(|| "000000000000".into());
        let aws_access_key_id = cli.aws_access_key_id.or(file.aws_access_key_id).unwrap_or_else(|| "awsem".into());
        let aws_secret_access_key = cli.aws_secret_access_key.or(file.aws_secret_access_key).unwrap_or_else(|| "awsem".into());
        let rustfs_access_key = cli.rustfs_access_key.or(file.rustfs_access_key).unwrap_or_else(|| "awsem".into());
        let rustfs_secret_key = cli.rustfs_secret_key.or(file.rustfs_secret_key).unwrap_or_else(|| "awsem".into());
        let spark_cpu = cli.spark_cpu.or(file.spark_cpu).unwrap_or_else(|| "500m".into());
        let spark_memory = cli.spark_memory.or(file.spark_memory).unwrap_or_else(|| "512Mi".into());
        let lambda_cpu = cli.lambda_cpu.or(file.lambda_cpu).unwrap_or_else(|| "250m".into());
        let lambda_memory = cli.lambda_memory.or(file.lambda_memory).unwrap_or_else(|| "256Mi".into());
        let emr_poll_interval = cli.emr_poll_interval.or(file.emr_poll_interval).unwrap_or(15);

        Self {
            port: Some(port), config_file: cli.config_file, db_path: Some(db_path),
            kubeconfig: cli.kubeconfig.or(file.kubeconfig),
            k8s_namespace: Some(k8s_namespace), rustfs_image: Some(rustfs_image),
            rustfs_pvc_size: Some(rustfs_pvc_size), s3_endpoint: cli.s3_endpoint.or(file.s3_endpoint),
            no_s3: Some(no_s3), no_emr: Some(no_emr), no_lambda: Some(no_lambda),
            lambda_runtime_image: cli.lambda_runtime_image.or(file.lambda_runtime_image),
            emr_spark_image: Some(emr_spark_image), data_dir, log_dir, log_level: Some(log_level),
            jwt_secret: Some(jwt_secret), host_ip: cli.host_ip.or(file.host_ip),
            region: Some(region), account_id: Some(account_id),
            aws_access_key_id: Some(aws_access_key_id), aws_secret_access_key: Some(aws_secret_access_key),
            default_pool_id: cli.default_pool_id.or(file.default_pool_id),
            spark_cpu: Some(spark_cpu), spark_memory: Some(spark_memory),
            lambda_cpu: Some(lambda_cpu), lambda_memory: Some(lambda_memory),
            emr_poll_interval: Some(emr_poll_interval),
            rustfs_access_key: Some(rustfs_access_key), rustfs_secret_key: Some(rustfs_secret_key),
        }
    }
}
