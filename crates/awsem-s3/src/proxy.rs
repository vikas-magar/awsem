use actix_web::{HttpRequest, HttpResponse, web};
use awsem_core::db::DbConn;
use std::collections::HashMap;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct S3State {
    pub rustfs_url: String,
    pub http_client: reqwest::Client,
    pub db: DbConn,
    pub event_bus: broadcast::Sender<awsem_events::BusEvent>,
}

fn to_reqwest_method(m: &actix_web::http::Method) -> reqwest::Method {
    match m.as_str() {
        "GET" => reqwest::Method::GET,
        "PUT" => reqwest::Method::PUT,
        "POST" => reqwest::Method::POST,
        "DELETE" => reqwest::Method::DELETE,
        "HEAD" => reqwest::Method::HEAD,
        "OPTIONS" => reqwest::Method::OPTIONS,
        "PATCH" => reqwest::Method::PATCH,
        _ => reqwest::Method::GET,
    }
}

fn copy_headers(req: &HttpRequest, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    let mut rb = rb;
    for (name, value) in req.headers() {
        if let Ok(v) = value.to_str() {
            rb = rb.header(name.as_str(), v);
        }
    }
    rb
}

pub async fn s3_handler(
    req: HttpRequest,
    body: bytes::Bytes,
    state: web::Data<S3State>,
) -> HttpResponse {
    let target_url = format!("{}{}", state.rustfs_url, req.uri());

    let reqwest_method = to_reqwest_method(req.method());
    let rb = state.http_client.request(reqwest_method, &target_url);
    let rb = copy_headers(&req, rb);

    let resp = match rb.body(body.to_vec()).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("S3 proxy error: {e}");
            return HttpResponse::InternalServerError().body("proxy error");
        }
    };

    let status_code = resp.status();
    let headers: HashMap<String, String> = resp
        .headers()
        .iter()
        .filter_map(|(n, v)| {
            v.to_str()
                .ok()
                .map(|s| (n.as_str().to_string(), s.to_string()))
        })
        .collect();

    let resp_body = resp.bytes().await.unwrap_or_default();

    if status_code.is_success() && req.method() == actix_web::http::Method::PUT {
        let path = req.path().to_string();
        let bucket_key: Vec<&str> = path.trim_start_matches('/').splitn(2, '/').collect();
        if bucket_key.len() == 2 && bucket_key[1].ends_with("_SUCCESS") {
            let bucket = bucket_key[0];
            let key = bucket_key[1];
            crate::event_detect::on_success(&state, bucket, key).await;
        }
    }

    let mut actix_resp = HttpResponse::build(
        actix_web::http::StatusCode::from_u16(status_code.as_u16())
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR),
    );
    for (name_str, value_str) in &headers {
        if let (Ok(n), Ok(v)) = (
            name_str.parse::<actix_web::http::header::HeaderName>(),
            value_str.parse::<actix_web::http::header::HeaderValue>(),
        ) {
            actix_resp.insert_header((n, v));
        }
    }
    actix_resp.body(resp_body)
}
