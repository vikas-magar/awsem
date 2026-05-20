use serde_json::{json, Value};

pub fn list_releases() -> Vec<Value> {
    vec![
        json!({
            "releaseLabel": "emr-7.1.0-latest",
            "state": "AVAILABLE",
            "applications": ["Spark", "Hive", "Hadoop"],
        }),
        json!({
            "releaseLabel": "emr-6.15.0",
            "state": "AVAILABLE",
            "applications": ["Spark", "Hive", "Hadoop"],
        }),
    ]
}
