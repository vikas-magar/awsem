pub mod cognito;
pub mod logs;
pub mod overview;
pub mod secrets;

use actix_web::web;
use awsem_core::db::DbConn;

#[derive(Clone)]
pub struct AdminState {
    pub db: DbConn,
    pub log_file: String,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/api")
            .route("/overview", web::get().to(overview::handle))
            .route("/cognito/users", web::get().to(cognito::list))
            .route("/cognito/delete", web::post().to(cognito::delete))
            .route("/secrets", web::get().to(secrets::list))
            .route("/secrets/create", web::post().to(secrets::create))
            .route("/secrets/delete", web::post().to(secrets::delete))
            .route("/logs", web::get().to(logs::handle)),
    );
}
