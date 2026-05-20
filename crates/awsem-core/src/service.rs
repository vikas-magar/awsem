use actix_web::web;

pub trait AwsService: Send + Sync {
    fn name(&self) -> &'static str;
    fn configure(&self, cfg: &mut web::ServiceConfig);
}
