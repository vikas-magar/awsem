use awsem_core::config::AppConfig;
use std::path::Path;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};

pub fn map_err(e: Box<dyn std::error::Error>) -> anyhow::Error {
    anyhow::anyhow!("{}", e)
}

pub fn init_logging(config: &AppConfig) -> (tracing_appender::non_blocking::WorkerGuard, String) {
    let log_dir = config.resolved_log_dir();
    let log_level = config.resolved_log_level();

    if !Path::new(log_dir).exists() && let Err(e) = std::fs::create_dir_all(log_dir) {
        eprintln!("Warning: failed to create log dir {log_dir}: {e}");
    }

    let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let log_file = format!("{log_dir}/awsem-{ts}.log");

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    let filename = format!("awsem-{ts}.log");
    let file_appender = tracing_appender::rolling::never(log_dir, &filename);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    Registry::default()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking).with_ansi(false).with_target(false))
        .with(tracing_subscriber::fmt::layer().with_ansi(true).with_target(true))
        .init();

    (guard, log_file)
}
