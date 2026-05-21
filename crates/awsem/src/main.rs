mod logging;

use actix_web::{web, App, HttpServer};
use awsem_core::config::AppConfig;
use awsem_core::db;
use awsem_core::k8s;
use std::path::Path;
use tokio::signal;
use tokio::sync::broadcast;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::load();
    let _log_guard = logging::init_logging(&config);

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

    let conn = db::open(db_path).map_err(logging::map_err)?;
    db::init_schema(&conn).map_err(logging::map_err)?;

    let kube_client = k8s::try_client().await;
    let (event_bus, _rx) = broadcast::channel(256);

    let s3_endpoint = if !config.resolved_no_s3() {
        if let Some(ref ep) = config.s3_endpoint {
            ep.clone()
        } else if let Some(ref client) = kube_client {
            let ns = config.resolved_k8s_namespace();
            k8s::ensure_namespace(client, ns).await.map_err(logging::map_err)?;
            let s3_cfg = awsem_s3::RustFsConfig::new(
                ns,
                config.rustfs_image.as_deref().unwrap_or("rustfs/rustfs:latest"),
                config.resolved_rustfs_pvc_size(),
            );
            awsem_s3::deploy::deploy(client, &s3_cfg).await.map_err(logging::map_err)?;
            awsem_s3::deploy::wait_ready(client, ns).await.map_err(logging::map_err)?;
            let pod_name = awsem_s3::deploy::get_pod_name(client, ns).await.map_err(logging::map_err)?;
            let port = awsem_s3::port_forward::port_forward(client, ns, &pod_name).await.map_err(logging::map_err)?;
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
        jwt_secret: config.resolved_jwt_secret().to_string(),
    };
    let secrets_state = awsem_secretsmanager::AppState { db: conn.clone() };
    let emr_state = awsem_emr::AppState {
        db: conn.clone(),
        event_bus: event_bus.clone(),
        k8s_client: kube_client.clone(),
        namespace: ns.clone(),
        emr_spark_image: config.emr_spark_image.clone(),
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

    let srv_handle = server.handle();
    tokio::spawn(async move {
        let mut term = signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("register SIGTERM handler");
        tokio::select! {
            _ = term.recv() => tracing::info!("Received SIGTERM, shutting down"),
            _ = signal::ctrl_c() => tracing::info!("Received SIGINT, shutting down"),
        }
        srv_handle.stop(true).await;
    });

    tracing::info!(port, "awsem started");
    if let Err(e) = server.await {
        tracing::error!("Server exited with error: {e}");
    }

    if let Some(ref client) = kube_client {
        let s3_cfg = awsem_s3::RustFsConfig::new(
            &ns, "", config.resolved_rustfs_pvc_size(),
        );
        if let Err(e) = awsem_s3::deploy::cleanup(client, &s3_cfg).await {
            tracing::warn!("Failed to clean up RustFS resources: {e}");
        }
    }

    Ok(())
}
