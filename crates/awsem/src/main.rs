use actix_web::{web, App, HttpServer};
use awsem_core::config::AppConfig;
use awsem_core::db;
use awsem_core::k8s;
use clap::Parser;
use tokio::sync::broadcast;
use tracing_subscriber::EnvFilter;

fn map_err(e: Box<dyn std::error::Error>) -> anyhow::Error {
    anyhow::anyhow!("{}", e)
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let config = AppConfig::parse();
    let conn = db::open(&config.db_path).map_err(map_err)?;
    db::init_schema(&conn).map_err(map_err)?;
    let kube_client = k8s::try_client().await;
    let (event_bus, _rx) = broadcast::channel(256);
    let s3_endpoint = if !config.no_s3 {
        if let Some(ref ep) = config.s3_endpoint {
            ep.clone()
        } else if let Some(ref client) = kube_client {
            k8s::ensure_namespace(client, &config.k8s_namespace).await.map_err(map_err)?;
            let s3_cfg = awsem_s3::RustFsConfig::new(
                &config.k8s_namespace,
                config.rustfs_image.as_deref().unwrap_or("rustfs/rustfs:latest"),
                &config.rustfs_pvc_size,
            );
            awsem_s3::deploy::deploy(client, &s3_cfg).await.map_err(map_err)?;
            awsem_s3::deploy::wait_ready(client, &config.k8s_namespace).await.map_err(map_err)?;
            let pod_name = awsem_s3::deploy::get_pod_name(client, &config.k8s_namespace).await.map_err(map_err)?;
            let port = awsem_s3::port_forward::port_forward(client, &config.k8s_namespace, &pod_name).await.map_err(map_err)?;
            format!("http://127.0.0.1:{port}")
        } else {
            tracing::warn!("No K8s client, S3 disabled");
            String::new()
        }
    } else {
        String::new()
    };
    let http_client = reqwest::Client::new();
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
        namespace: config.k8s_namespace.clone(),
    };
    let lambda_state = awsem_lambda::AppState {
        db: conn.clone(),
        event_bus: event_bus.clone(),
        data_dir: config.data_dir.clone(),
    };
    if !config.no_emr {
        let db = conn.clone();
        let kc = kube_client.clone();
        let url = s3_endpoint.clone();
        let client = http_client.clone();
        let bus = event_bus.clone();
        tokio::spawn(async move {
            awsem_emr::job_watcher::watch_jobs(db, kc, url, client, bus).await;
        });
    }
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
    .bind(("0.0.0.0", config.port))?
    .run();
    tracing::info!("awsem starting on port {}", config.port);
    let _ = server.await;
    if let Some(ref client) = kube_client {
        let s3_cfg = awsem_s3::RustFsConfig::new(
            &config.k8s_namespace, "", &config.rustfs_pvc_size,
        );
        let _ = awsem_s3::deploy::cleanup(client, &s3_cfg).await;
    }
    Ok(())
}
