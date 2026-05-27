use crate::aws::AwsConfig;
use clap::Parser;
use serde::Deserialize;

#[derive(Parser, Clone)]
pub struct AppConfig {
    #[arg(long)]
    pub port: Option<u16>,
    #[arg(long)]
    pub config_file: Option<String>,
    #[arg(long)]
    pub db_path: Option<String>,
    #[arg(long)]
    pub kubeconfig: Option<String>,
    #[arg(long)]
    pub k8s_namespace: Option<String>,
    #[arg(long)]
    pub rustfs_image: Option<String>,
    #[arg(long)]
    pub rustfs_pvc_size: Option<String>,
    #[arg(long)]
    pub s3_endpoint: Option<String>,
    #[arg(long)]
    pub no_s3: Option<bool>,
    #[arg(long)]
    pub no_emr: Option<bool>,
    #[arg(long)]
    pub no_lambda: Option<bool>,
    #[arg(long)]
    pub lambda_runtime_image: Option<String>,
    #[arg(long)]
    pub emr_spark_image: Option<String>,
    #[arg(long)]
    pub data_dir: Option<String>,
    #[arg(long)]
    pub log_dir: Option<String>,
    #[arg(long)]
    pub log_level: Option<String>,
    #[arg(long)]
    pub jwt_secret: Option<String>,
    #[arg(long)]
    pub host_ip: Option<String>,
    #[arg(long)]
    pub region: Option<String>,
    #[arg(long)]
    pub account_id: Option<String>,
    #[arg(long)]
    pub aws_access_key_id: Option<String>,
    #[arg(long)]
    pub aws_secret_access_key: Option<String>,
    #[arg(long)]
    pub default_pool_id: Option<String>,
    #[arg(long)]
    pub spark_cpu: Option<String>,
    #[arg(long)]
    pub spark_memory: Option<String>,
    #[arg(long)]
    pub lambda_cpu: Option<String>,
    #[arg(long)]
    pub lambda_memory: Option<String>,
    #[arg(long)]
    pub emr_poll_interval: Option<u64>,
    #[arg(long)]
    pub rustfs_access_key: Option<String>,
    #[arg(long)]
    pub rustfs_secret_key: Option<String>,
}

#[derive(Deserialize, Clone, Default)]
pub struct ConfigFile {
    pub port: Option<u16>,
    pub db_path: Option<String>,
    pub kubeconfig: Option<String>,
    pub k8s_namespace: Option<String>,
    pub rustfs_image: Option<String>,
    pub rustfs_pvc_size: Option<String>,
    pub s3_endpoint: Option<String>,
    pub no_s3: Option<bool>,
    pub no_emr: Option<bool>,
    pub no_lambda: Option<bool>,
    pub lambda_runtime_image: Option<String>,
    pub emr_spark_image: Option<String>,
    pub data_dir: Option<String>,
    pub log_dir: Option<String>,
    pub log_level: Option<String>,
    pub jwt_secret: Option<String>,
    pub host_ip: Option<String>,
    pub region: Option<String>,
    pub account_id: Option<String>,
    pub aws_access_key_id: Option<String>,
    pub aws_secret_access_key: Option<String>,
    pub default_pool_id: Option<String>,
    pub spark_cpu: Option<String>,
    pub spark_memory: Option<String>,
    pub lambda_cpu: Option<String>,
    pub lambda_memory: Option<String>,
    pub emr_poll_interval: Option<u64>,
    pub rustfs_access_key: Option<String>,
    pub rustfs_secret_key: Option<String>,
}

impl AppConfig {
    pub fn load() -> Self {
        let cli = Self::parse();
        let config_path = cli.config_file.clone().or_else(crate::defaults::default_config_path);
        let file = config_path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| toml::from_str::<ConfigFile>(s.as_str()).ok()).unwrap_or_default();
        Self::merge(cli, file)
    }
}

impl AppConfig {
    pub fn resolved_port(&self) -> u16 { self.port.unwrap_or(4566) }
    pub fn resolved_db_path(&self) -> &str { self.db_path.as_deref().unwrap_or("./awsem.db") }
    pub fn resolved_k8s_namespace(&self) -> &str { self.k8s_namespace.as_deref().unwrap_or("awsem") }
    pub fn resolved_rustfs_pvc_size(&self) -> &str { self.rustfs_pvc_size.as_deref().unwrap_or("10Gi") }
    pub fn resolved_no_s3(&self) -> bool { self.no_s3.unwrap_or(false) }
    pub fn resolved_no_emr(&self) -> bool { self.no_emr.unwrap_or(false) }
    pub fn resolved_no_lambda(&self) -> bool { self.no_lambda.unwrap_or(false) }
    pub fn resolved_data_dir(&self) -> Option<&str> { self.data_dir.as_deref() }
    pub fn resolved_log_dir(&self) -> &str { self.log_dir.as_deref().unwrap_or("./logs") }
    pub fn resolved_log_level(&self) -> &str { self.log_level.as_deref().unwrap_or("info") }
    pub fn resolved_jwt_secret(&self) -> &str { self.jwt_secret.as_deref().unwrap_or("awsem-dev-secret") }
    pub fn resolved_host_ip(&self) -> &str { self.host_ip.as_deref().unwrap_or("127.0.0.1") }
    pub fn resolved_rustfs_image(&self) -> &str { self.rustfs_image.as_deref().unwrap_or("rustfs/rustfs:latest") }
    pub fn resolved_emr_spark_image(&self) -> &str { self.emr_spark_image.as_deref().unwrap_or("spark-s3a:latest") }
    pub fn resolved_region(&self) -> &str { self.region.as_deref().unwrap_or("us-east-1") }
    pub fn resolved_account_id(&self) -> &str { self.account_id.as_deref().unwrap_or("000000000000") }
    pub fn resolved_aws_access_key_id(&self) -> &str { self.aws_access_key_id.as_deref().unwrap_or("awsem") }
    pub fn resolved_aws_secret_access_key(&self) -> &str { self.aws_secret_access_key.as_deref().unwrap_or("awsem") }
    pub fn resolved_default_pool_id(&self) -> String { self.default_pool_id.clone().unwrap_or_else(|| format!("{}_default", self.resolved_region())) }
    pub fn resolved_spark_cpu(&self) -> &str { self.spark_cpu.as_deref().unwrap_or("500m") }
    pub fn resolved_spark_memory(&self) -> &str { self.spark_memory.as_deref().unwrap_or("512Mi") }
    pub fn resolved_lambda_cpu(&self) -> &str { self.lambda_cpu.as_deref().unwrap_or("250m") }
    pub fn resolved_lambda_memory(&self) -> &str { self.lambda_memory.as_deref().unwrap_or("256Mi") }
    pub fn resolved_emr_poll_interval(&self) -> u64 { self.emr_poll_interval.unwrap_or(15) }
    pub fn resolved_rustfs_access_key(&self) -> &str { self.rustfs_access_key.as_deref().unwrap_or("awsem") }
    pub fn resolved_rustfs_secret_key(&self) -> &str { self.rustfs_secret_key.as_deref().unwrap_or("awsem") }
    pub fn resolved_aws_config(&self) -> AwsConfig {
        AwsConfig::new(self.resolved_region(), self.resolved_account_id(), self.resolved_aws_access_key_id(), self.resolved_aws_secret_access_key())
    }
}
