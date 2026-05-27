pub mod handlers;
pub mod queries;
pub mod read;
pub mod store;

use actix_web::guard::{Guard, GuardContext};
use actix_web::web;
use awsem_core::db::DbConn;

#[derive(Clone)]
pub struct AppState {
    pub db: DbConn,
    pub aws: awsem_core::aws::AwsConfig,
}

pub struct SecretsGuard;

impl Guard for SecretsGuard {
    fn check(&self, ctx: &GuardContext<'_>) -> bool {
        ctx.head()
            .headers()
            .get("X-Amz-Target")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.starts_with("secretsmanager"))
            .unwrap_or(false)
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/", web::post().guard(SecretsGuard).to(handlers::handle));
}
