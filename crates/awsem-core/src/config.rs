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
    pub data_dir: Option<String>,

    #[arg(long)]
    pub log_dir: Option<String>,

    #[arg(long)]
    pub log_level: Option<String>,
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
    pub data_dir: Option<String>,
    pub log_dir: Option<String>,
    pub log_level: Option<String>,
}

impl AppConfig {
    pub fn load() -> Self {
        let cli = Self::parse();

        let config_path = cli
            .config_file
            .clone()
            .or_else(default_config_path);

        let file = config_path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| toml::from_str::<ConfigFile>(s.as_str()).ok())
            .unwrap_or_default();

        Self::merge(cli, file)
    }

    fn merge(cli: Self, file: ConfigFile) -> Self {
        let port = cli.port.or(file.port).unwrap_or(4566);

        let db_path = cli
            .db_path
            .or(file.db_path)
            .unwrap_or_else(|| default_db_path(port));

        let k8s_namespace = cli
            .k8s_namespace
            .or(file.k8s_namespace)
            .unwrap_or_else(|| "awsem".into());

        let rustfs_pvc_size = cli
            .rustfs_pvc_size
            .or(file.rustfs_pvc_size)
            .unwrap_or_else(|| "10Gi".into());

        let no_s3 = cli.no_s3.or(file.no_s3).unwrap_or(false);
        let no_emr = cli.no_emr.or(file.no_emr).unwrap_or(false);
        let no_lambda = cli.no_lambda.or(file.no_lambda).unwrap_or(false);

        let data_dir = cli
            .data_dir
            .or(file.data_dir)
            .or_else(default_data_dir);

        let log_dir = cli
            .log_dir
            .or(file.log_dir)
            .or_else(default_log_dir);

        let log_level = cli
            .log_level
            .or(file.log_level)
            .unwrap_or_else(|| "info".into());

        Self {
            port: Some(port),
            config_file: cli.config_file,
            db_path: Some(db_path),
            kubeconfig: cli.kubeconfig.or(file.kubeconfig),
            k8s_namespace: Some(k8s_namespace),
            rustfs_image: cli.rustfs_image.or(file.rustfs_image),
            rustfs_pvc_size: Some(rustfs_pvc_size),
            s3_endpoint: cli.s3_endpoint.or(file.s3_endpoint),
            no_s3: Some(no_s3),
            no_emr: Some(no_emr),
            no_lambda: Some(no_lambda),
            lambda_runtime_image: cli.lambda_runtime_image.or(file.lambda_runtime_image),
            data_dir,
            log_dir,
            log_level: Some(log_level),
        }
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
}

fn default_config_path() -> Option<String> {
    let base = dirs::config_dir()?;
    let path = base.join("awsem").join("config.toml");
    Some(path.to_string_lossy().into())
}

fn default_data_dir() -> Option<String> {
    let base = dirs::config_dir()?;
    let path = base.join("awsem").join("data");
    Some(path.to_string_lossy().into())
}

fn default_log_dir() -> Option<String> {
    let base = dirs::config_dir()?;
    let path = base.join("awsem").join("logs");
    Some(path.to_string_lossy().into())
}

fn default_db_path(port: u16) -> String {
    if let Some(base) = dirs::config_dir() {
        let path = base.join("awsem").join("awsem.db");
        path.to_string_lossy().into()
    } else {
        format!("awsem-{port}.db")
    }
}
