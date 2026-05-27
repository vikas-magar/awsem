pub mod admin;
pub mod emr_classic;
pub mod emr_classic_steps;
pub mod job_runs;
pub mod job_watcher;
pub mod k8s_job;
pub mod releases;
pub mod spark_params;
pub mod spawner;
pub mod virtual_cluster;

use actix_web::guard::{Guard, GuardContext};
use actix_web::web;
use awsem_core::db::DbConn;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub db: DbConn,
    pub event_bus: broadcast::Sender<awsem_events::BusEvent>,
    pub k8s_client: Option<kube::Client>,
    pub namespace: String,
    pub emr_spark_image: String,
    pub awsem_endpoint: String,
    pub aws: awsem_core::aws::AwsConfig,
    pub rustfs_access_key: String,
    pub rustfs_secret_key: String,
}

pub struct EmrClassicGuard;

impl Guard for EmrClassicGuard {
    fn check(&self, ctx: &GuardContext<'_>) -> bool {
        ctx.head()
            .headers()
            .get("X-Amz-Target")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.starts_with("ElasticMapReduce"))
            .unwrap_or(false)
    }
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
    cfg.route("/", web::post().guard(EmrClassicGuard).to(emr_classic::handle));
    cfg.route("/releases", web::get().to(releases::handle_list));
    cfg.service(
        web::scope("/admin/api/emr")
            .route("/jobs/{id}/complete", web::post().to(admin::job_complete))
            .route("/jobs/{id}/fail", web::post().to(admin::job_fail)),
    );
}
