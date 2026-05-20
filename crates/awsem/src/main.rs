use actix_web::{web, App, HttpServer};
use awsem_core::config::AppConfig;
use awsem_core::db;
use awsem_core::k8s;
use std::path::Path;
use tokio::sync::broadcast;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};

fn map_err(e: Box<dyn std::error::Error>) -> anyhow::Error {
    anyhow::anyhow!("{}", e)
}

fn init_logging(config: &AppConfig) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let log_dir = config.resolved_log_dir();
    let log_level = config.resolved_log_level();

    if !Path::new(log_dir).exists() {
        let _ = std::fs::create_dir_all(log_dir);
    }

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    let file_appender = tracing_appender::rolling::daily(log_dir, "awsem.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    Registry::default()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(false),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .with_target(true),
        )
        .init();

    Some(guard)
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::load();
    let _log_guard = init_logging(&config);

    if let Some(dir) = config.resolved_data_dir()
        && !Path::new(dir).exists()
    {
        std::fs::create_dir_all(dir)?;
    }

    let db_path = config.resolved_db_path();
    if let Some(parent) = Path::new(db_path).parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent)?;
    }

    let conn = db::open(db_path).map_err(map_err)?;
    db::init_schema(&conn).map_err(map_err)?;

    let kube_client = k8s::try_client().await;
    let (event_bus, _rx) = broadcast::channel(256);

    let s3_endpoint = if !config.resolved_no_s3() {
        if let Some(ref ep) = config.s3_endpoint {
            ep.clone()
        } else if let Some(ref client) = kube_client {
            let ns = config.resolved_k8s_namespace();
            k8s::ensure_namespace(client, ns).await.map_err(map_err)?;
            let s3_cfg = awsem_s3::RustFsConfig::new(
                ns,
                config.rustfs_image.as_deref().unwrap_or("rustfs/rustfs:latest"),
                config.resolved_rustfs_pvc_size(),
            );
            awsem_s3::deploy::deploy(client, &s3_cfg).await.map_err(map_err)?;
            awsem_s3::deploy::wait_ready(client, ns).await.map_err(map_err)?;
            let pod_name = awsem_s3::deploy::get_pod_name(client, ns).await.map_err(map_err)?;
            let port = awsem_s3::port_forward::port_forward(client, ns, &pod_name).await.map_err(map_err)?;
            format!("http://127.0.0.1:{port}")
        } else {
            tracing::warn!("No K8s client, S3 disabled");
            String::new()
        }
    } else {
        String::new()
    };

    let http_client = reqwest::Client::new();
    let ns = config.resolved_k8s_namespace().to_string();
    let port = config.resolved_port();

    let s3_state = awsem_s3::S3State {
        rustfs_url: s3_endpoint.clone(),
        http_client: http_client.clone(),
        db: conn.clone(),
        event_bus: event_bus.clone(),
    };
    let cognito_state = awsem_cognito::AppState {
        db: conn.clone(),
        jwt_secret: "awsem-dev-secret".into(),
    };
    let secrets_state = awsem_secretsmanager::AppState { db: conn.clone() };
    let emr_state = awsem_emr::AppState {
        db: conn.clone(),
        event_bus: event_bus.clone(),
        k8s_client: kube_client.clone(),
        namespace: ns.clone(),
    };
    let lambda_state = awsem_lambda::AppState {
        db: conn.clone(),
        event_bus: event_bus.clone(),
        data_dir: config.resolved_data_dir().map(String::from),
        kube_client: kube_client.clone(),
        namespace: ns.clone(),
        lambda_runtime_image: config.lambda_runtime_image.clone(),
    };

    if !config.resolved_no_emr() {
        let db = conn.clone();
        let kc = kube_client.clone();
        let url = s3_endpoint.clone();
        let client = http_client.clone();
        let bus = event_bus.clone();
        tokio::spawn(async move {
            awsem_emr::job_watcher::watch_jobs(db, kc, url, client, bus).await;
        });
    }

    let lambda_trigger_state = lambda_state.clone();
    tokio::spawn(async move {
        awsem_lambda::trigger::listen(lambda_trigger_state).await;
    });

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(cognito_state.clone()))
            .app_data(web::Data::new(secrets_state.clone()))
            .app_data(web::Data::new(emr_state.clone()))
            .app_data(web::Data::new(lambda_state.clone()))
            .app_data(web::Data::new(s3_state.clone()))
            .configure(awsem_cognito::configure)
            .configure(awsem_secretsmanager::configure)
            .configure(awsem_emr::configure)
            .configure(awsem_lambda::configure)
            .configure(awsem_s3::configure)
    })
    .bind(("0.0.0.0", port))?
    .run();

    tracing::info!(port, "awsem started");
    let _ = server.await;

    if let Some(ref client) = kube_client {
        let s3_cfg = awsem_s3::RustFsConfig::new(
            &ns, "", config.resolved_rustfs_pvc_size(),
        );
        let _ = awsem_s3::deploy::cleanup(client, &s3_cfg).await;
    }

    Ok(())
}
