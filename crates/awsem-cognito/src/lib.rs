pub mod admin;
pub mod handlers;
pub mod jwt;
pub mod store;

use actix_web::guard::{Guard, GuardContext};
use actix_web::web;
use awsem_core::db::DbConn;

#[derive(Clone)]
pub struct AppState {
    pub db: DbConn,
    pub jwt_secret: String,
    pub aws: awsem_core::aws::AwsConfig,
}

pub struct CognitoGuard;

impl Guard for CognitoGuard {
    fn check(&self, ctx: &GuardContext<'_>) -> bool {
        ctx.head()
            .headers()
            .get("X-Amz-Target")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.starts_with("AWSCognitoIdentityProviderService"))
            .unwrap_or(false)
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/", web::post().guard(CognitoGuard).to(handlers::handle));
}
