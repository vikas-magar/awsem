use awsem_core::config::AppConfig;
use std::path::Path;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};

pub fn map_err(e: Box<dyn std::error::Error>) -> anyhow::Error {
    anyhow::anyhow!("{}", e)
}

pub fn init_logging(config: &AppConfig) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let log_dir = config.resolved_log_dir();
    let log_level = config.resolved_log_level();

    if !Path::new(log_dir).exists()
        && let Err(e) = std::fs::create_dir_all(log_dir)
    {
        eprintln!("Warning: failed to create log dir {log_dir}: {e}");
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
