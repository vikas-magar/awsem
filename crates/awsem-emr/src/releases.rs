use actix_web::HttpResponse;
use serde_json::json;

pub async fn handle_list() -> HttpResponse {
    HttpResponse::Ok().json(json!({"releases": [
        {"releaseLabel": "emr-7.1.0-latest", "state": "AVAILABLE", "applications": ["Spark", "Hive", "Hadoop"]},
        {"releaseLabel": "emr-6.15.0", "state": "AVAILABLE", "applications": ["Spark", "Hive", "Hadoop"]},
    ]}))
}
