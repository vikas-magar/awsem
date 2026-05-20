pub mod config;
pub mod deploy;
pub mod event_detect;
pub mod notification;
pub mod port_forward;
pub mod proxy;

pub use config::RustFsConfig;
pub use proxy::S3State;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/{tail:.*}", web::route().to(proxy::s3_handler));
}
