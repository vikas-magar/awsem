pub mod create;
pub mod event_source;
pub mod execute;
pub mod extract;
pub mod functions;
pub mod invoke;
pub mod name;
pub mod trigger;

use actix_web::web;
use awsem_core::db::DbConn;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub db: DbConn,
    pub event_bus: broadcast::Sender<awsem_events::BusEvent>,
    pub data_dir: Option<String>,
    pub kube_client: Option<kube::Client>,
    pub namespace: String,
    pub lambda_runtime_image: Option<String>,
    pub aws: awsem_core::aws::AwsConfig,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/2015-03-31")
            .route("/functions", web::post().to(create::create))
            .route("/functions", web::get().to(functions::list))
            .route("/functions/{name}", web::get().to(functions::get))
            .route("/functions/{name}", web::delete().to(functions::delete_fn))
            .route(
                "/functions/{name}/invocations",
                web::post().to(invoke::handle),
            )
            .route(
                "/event-source-mappings",
                web::post().to(event_source::create),
            )
            .route("/event-source-mappings", web::get().to(event_source::list))
            .route(
                "/event-source-mappings/{uuid}",
                web::delete().to(event_source::delete),
            ),
    );
}
