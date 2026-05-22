use crate::config::{AppConfig, ConfigFile};

pub fn default_config_path() -> Option<String> {
    let base = dirs::config_dir()?;
    Some(
        base.join("awsem")
            .join("config.toml")
            .to_string_lossy()
            .into(),
    )
}

pub fn default_data_dir() -> Option<String> {
    let base = dirs::config_dir()?;
    Some(base.join("awsem").join("data").to_string_lossy().into())
}

pub fn default_log_dir() -> Option<String> {
    let base = dirs::config_dir()?;
    Some(base.join("awsem").join("logs").to_string_lossy().into())
}

pub fn default_db_path(port: u16) -> String {
    if let Some(base) = dirs::config_dir() {
        base.join("awsem").join("awsem.db").to_string_lossy().into()
    } else {
        format!("awsem-{port}.db")
    }
}

pub fn default_jwt_secret() -> String {
    uuid::Uuid::new_v4().to_string()
}

impl AppConfig {
    pub fn merge(cli: Self, file: ConfigFile) -> Self {
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
        let data_dir = cli.data_dir.or(file.data_dir).or_else(default_data_dir);
        let log_dir = cli.log_dir.or(file.log_dir).or_else(default_log_dir);
        let log_level = cli
            .log_level
            .or(file.log_level)
            .unwrap_or_else(|| "info".into());
        let jwt_secret = cli
            .jwt_secret
            .clone()
            .or(file.jwt_secret)
            .unwrap_or_else(default_jwt_secret);

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
            emr_spark_image: cli.emr_spark_image.or(file.emr_spark_image.clone()),
            data_dir,
            log_dir,
            log_level: Some(log_level),
            jwt_secret: Some(jwt_secret),
            host_ip: cli.host_ip.or(file.host_ip),
        }
    }
}
