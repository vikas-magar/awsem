use serde_json::json;

pub async fn run(_name: &str, _runtime: &str, _handler: &str) -> String {
    json!({"statusCode": 200, "body": "Hello from awsem Lambda!"}).to_string()
}
