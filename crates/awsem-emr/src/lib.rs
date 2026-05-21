pub mod job_runs;
pub mod job_watcher;
pub mod k8s_job;
pub mod releases;
pub mod spark_params;
pub mod virtual_cluster;

use actix_web::web;
use awsem_core::db::DbConn;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub db: DbConn,
    pub event_bus: broadcast::Sender<awsem_events::BusEvent>,
    pub k8s_client: Option<kube::Client>,
    pub namespace: String,
    pub emr_spark_image: Option<String>,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/virtualclusters")
            .route("", web::post().to(virtual_cluster::create))
            .route("", web::get().to(virtual_cluster::list))
            .route("/{id}", web::delete().to(virtual_cluster::delete))
            .route("/{vc_id}/jobruns", web::post().to(k8s_job::start_job_run))
            .route("/{vc_id}/jobruns", web::get().to(job_runs::list))
            .route("/{vc_id}/jobruns/{jr_id}", web::get().to(job_runs::describe))
            .route("/{vc_id}/jobruns/{jr_id}", web::delete().to(job_runs::cancel)),
    );
    cfg.route("/releases", web::get().to(releases::handle_list));
}
