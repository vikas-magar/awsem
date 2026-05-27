mod logging;

use actix_web::{App, HttpServer, web};
use awsem_core::config::AppConfig;
use awsem_core::db;
use awsem_core::k8s;
use std::path::Path;
use tokio::signal;
use tokio::sync::broadcast;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let config = AppConfig::load();
    let (_log_guard, log_file) = logging::init_logging(&config);

    if let Some(dir) = config.resolved_data_dir() && !Path::new(dir).exists() { std::fs::create_dir_all(dir)?; }
    let db_path = config.resolved_db_path();
    if let Some(parent) = Path::new(db_path).parent() && !parent.exists() { std::fs::create_dir_all(parent)?; }

    let conn = db::open(db_path).map_err(logging::map_err)?;
    db::init_schema(&conn).map_err(logging::map_err)?;

    let kube_client = k8s::try_client().await;
    let (event_bus, _rx) = broadcast::channel(256);

    let s3_endpoint = if !config.resolved_no_s3() {
        if let Some(ep) = config.resolved_s3_endpoint() { ep.to_string() }
        else if let Some(ref client) = kube_client {
            let s3_cfg = awsem_s3::RustFsConfig::new(config.resolved_k8s_namespace(), config.resolved_rustfs_image(), config.resolved_rustfs_pvc_size(), config.resolved_rustfs_access_key(), config.resolved_rustfs_secret_key());
            awsem_s3::deploy::ensure(client, &s3_cfg).await.map_err(logging::map_err)?
        } else { tracing::warn!("No K8s client, S3 disabled"); String::new() }
    } else { String::new() };

    let http_client = reqwest::Client::new();
    let s3_client = reqwest::Client::builder().pool_max_idle_per_host(0).build().map_err(|e| anyhow::anyhow!("{e}"))?;
    let ns = config.resolved_k8s_namespace().to_string();
    let port = config.resolved_port();
    let host_ip = config.resolved_host_ip();

    let awsem_endpoint = format!("http://{host_ip}:{port}");
    let aws = config.resolved_aws_config();

    let s3_state = awsem_s3::S3State { rustfs_url: s3_endpoint.clone(), http_client: s3_client, db: conn.clone(), event_bus: event_bus.clone() };
    let cognito_state = awsem_cognito::AppState { db: conn.clone(), jwt_secret: config.resolved_jwt_secret(), aws: aws.clone() };
    let secrets_state = awsem_secretsmanager::AppState { db: conn.clone(), aws: aws.clone() };
    let emr_state = awsem_emr::AppState { db: conn.clone(), event_bus: event_bus.clone(), k8s_client: kube_client.clone(), namespace: ns.clone(), emr_spark_image: config.resolved_emr_spark_image().to_string(), awsem_endpoint, aws: aws.clone(), rustfs_access_key: config.resolved_rustfs_access_key().to_string(), rustfs_secret_key: config.resolved_rustfs_secret_key().to_string() };
    let lambda_state = awsem_lambda::AppState { db: conn.clone(), event_bus: event_bus.clone(), data_dir: config.resolved_data_dir().map(String::from), kube_client: kube_client.clone(), namespace: ns.clone(), lambda_runtime_image: config.lambda_runtime_image.clone(), aws: aws.clone() };

    if !config.resolved_no_emr() {
        let (db, kc, url, client, bus, a) = (conn.clone(), kube_client.clone(), s3_endpoint.clone(), http_client.clone(), event_bus.clone(), aws.clone());
        tokio::spawn(async move { awsem_emr::job_watcher::watch_jobs(db, kc, url, client, bus, a).await; });
    }

    let admin_state = awsem_care::AdminState { db: conn.clone(), log_file: log_file.clone(), aws: aws.clone() };
    if !config.resolved_no_lambda() {
        let lambda_trigger_state = lambda_state.clone();
        tokio::spawn(async move { awsem_lambda::trigger::listen(lambda_trigger_state).await; });
    }

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(admin_state.clone()))
            .app_data(web::Data::new(cognito_state.clone()))
            .app_data(web::Data::new(secrets_state.clone()))
            .app_data(web::Data::new(emr_state.clone()))
            .app_data(web::Data::new(lambda_state.clone()))
            .app_data(web::Data::new(s3_state.clone()))
            .configure(awsem_care::configure)
            .configure(awsem_cognito::configure)
            .configure(awsem_secretsmanager::configure)
            .configure(awsem_emr::configure)
            .configure(awsem_lambda::configure)
            .configure(awsem_s3::configure)
    }).bind(("0.0.0.0", port))?.run();

    let srv_handle = server.handle();
    tokio::spawn(async move {
        let mut term = signal::unix::signal(signal::unix::SignalKind::terminate()).expect("register SIGTERM handler");
        tokio::select! { _ = term.recv() => tracing::info!("SIGTERM"), _ = signal::ctrl_c() => tracing::info!("SIGINT") }
        srv_handle.stop(true).await;
    });

    tracing::info!(port, "awsem started");
    eprintln!("log_file={log_file}");
    if let Err(e) = server.await { tracing::error!("Server exited with error: {e}"); }

    if let Some(ref client) = kube_client {
        let s3_cfg = awsem_s3::RustFsConfig::new(&ns, "", config.resolved_rustfs_pvc_size(), config.resolved_rustfs_access_key(), config.resolved_rustfs_secret_key());
        if let Err(e) = awsem_s3::deploy::cleanup(client, &s3_cfg).await { tracing::warn!("Failed to clean up RustFS: {e}"); }
    }

    Ok(())
}
