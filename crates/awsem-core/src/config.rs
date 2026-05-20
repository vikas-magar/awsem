#[derive(clap::Parser, Clone)]
pub struct AppConfig {
    #[arg(default_value = "4566")]
    pub port: u16,

    #[arg(long, default_value = "./awsem.db")]
    pub db_path: String,

    #[arg(long)]
    pub kubeconfig: Option<String>,

    #[arg(long, default_value = "awsem")]
    pub k8s_namespace: String,

    #[arg(long)]
    pub rustfs_image: Option<String>,

    #[arg(long, default_value = "10Gi")]
    pub rustfs_pvc_size: String,

    #[arg(long)]
    pub s3_endpoint: Option<String>,

    #[arg(long, default_value = "false")]
    pub no_s3: bool,

    #[arg(long, default_value = "false")]
    pub no_emr: bool,

    #[arg(long, default_value = "false")]
    pub no_lambda: bool,

    #[arg(long)]
    pub lambda_runtime_image: Option<String>,

    #[arg(long)]
    pub data_dir: Option<String>,
}
