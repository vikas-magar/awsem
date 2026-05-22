use crate::proxy::S3State;
use actix_web::{HttpResponse, web};
use serde_json::{Value, json};
use std::collections::HashMap;

pub async fn list_buckets(state: web::Data<S3State>) -> HttpResponse {
    let url = format!("{}/", state.rustfs_url);
    let resp = match state.http_client.get(&url).send().await {
        Ok(r) => r, Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("S3 proxy: {e}")})),
    };
    let xml = match resp.text().await { Ok(t) => t, Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("{e}")})), };
    let buckets: Vec<Value> = xml.split("<Bucket>").skip(1).filter_map(|s| {
        let name = s.split("<Name>").nth(1)?.split("</Name>").next()?;
        Some(json!({"Name": name}))
    }).collect();
    HttpResponse::Ok().json(json!({"Buckets": buckets}))
}

pub async fn list_objects(state: web::Data<S3State>, query: web::Query<HashMap<String, String>>) -> HttpResponse {
    let bucket = match query.get("bucket") { Some(b) => b, None => return HttpResponse::BadRequest().json(json!({"error": "missing bucket"})), };
    let url = format!("{}/{bucket}?list-type=2", state.rustfs_url);
    let resp = match state.http_client.get(&url).send().await {
        Ok(r) => r, Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("S3 proxy: {e}")})),
    };
    let xml = match resp.text().await { Ok(t) => t, Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("{e}")})), };
    let contents: Vec<Value> = xml.split("<Contents>").skip(1).filter_map(|s| {
        let key = s.split("<Key>").nth(1)?.split("</Key>").next()?;
        let size: i64 = s.split("<Size>").nth(1)?.split("</Size>").next()?.parse().unwrap_or(0);
        Some(json!({"Key": key, "Size": size}))
    }).collect();
    HttpResponse::Ok().json(json!({"Contents": contents}))
}
