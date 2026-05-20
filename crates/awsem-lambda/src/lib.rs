pub mod event_source;
pub mod execute;
pub mod functions;
pub mod invoke;

use actix_web::guard::{Guard, GuardContext};
use actix_web::web;
use awsem_core::db::DbConn;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub db: DbConn,
    pub event_bus: broadcast::Sender<awsem_events::BusEvent>,
    pub data_dir: Option<String>,
}

pub struct LambdaGuard;

impl Guard for LambdaGuard {
    fn check(&self, ctx: &GuardContext<'_>) -> bool {
        ctx.head()
            .headers()
            .get("X-Amz-Target")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.starts_with("AWSLambda") || v.starts_with("Lambda"))
            .unwrap_or(false)
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/", web::post().guard(LambdaGuard).to(functions::handle));
}
