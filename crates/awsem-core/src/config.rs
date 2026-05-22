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
}

impl AppConfig {
    pub fn load() -> Self {
        let cli = Self::parse();
        let config_path = cli
            .config_file
            .clone()
            .or_else(crate::defaults::default_config_path);
        let file = config_path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| toml::from_str::<ConfigFile>(s.as_str()).ok())
            .unwrap_or_default();
        Self::merge(cli, file)
    }
}

impl AppConfig {
    pub fn resolved_port(&self) -> u16 {
        self.port.unwrap_or(4566)
    }
    pub fn resolved_db_path(&self) -> &str {
        self.db_path.as_deref().unwrap_or("./awsem.db")
    }
    pub fn resolved_k8s_namespace(&self) -> &str {
        self.k8s_namespace.as_deref().unwrap_or("awsem")
    }
    pub fn resolved_rustfs_pvc_size(&self) -> &str {
        self.rustfs_pvc_size.as_deref().unwrap_or("10Gi")
    }
    pub fn resolved_no_s3(&self) -> bool {
        self.no_s3.unwrap_or(false)
    }
    pub fn resolved_no_emr(&self) -> bool {
        self.no_emr.unwrap_or(false)
    }
    pub fn resolved_no_lambda(&self) -> bool {
        self.no_lambda.unwrap_or(false)
    }
    pub fn resolved_data_dir(&self) -> Option<&str> {
        self.data_dir.as_deref()
    }
    pub fn resolved_log_dir(&self) -> &str {
        self.log_dir.as_deref().unwrap_or("./logs")
    }
    pub fn resolved_log_level(&self) -> &str {
        self.log_level.as_deref().unwrap_or("info")
    }
    pub fn resolved_jwt_secret(&self) -> &str {
        self.jwt_secret.as_deref().unwrap_or("awsem-dev-secret")
    }
    pub fn resolved_host_ip(&self) -> &str {
        self.host_ip.as_deref().unwrap_or("127.0.0.1")
    }
}
