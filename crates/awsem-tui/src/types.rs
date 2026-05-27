#[derive(PartialEq)]
pub struct S3Bucket {
    pub name: String,
    pub created: String,
    pub objects: String,
    pub size: String,
}

#[derive(Clone, PartialEq)]
pub struct S3Object {
    pub key: String,
    pub size: i64,
    pub size_str: String,
    pub modified: String,
    pub storage_class: String,
    pub is_folder: bool,
}

pub struct CognitoUser {
    pub username: String,
    pub status: String,
    pub email: String,
    pub created: String,
}

pub struct SecretEntry {
    pub name: String,
    pub arn: String,
    pub description: String,
    pub last_changed: String,
    pub rotation: String,
    pub status: String,
}

pub struct LambdaFn {
    pub name: String,
    pub runtime: String,
    pub timeout: i64,
    pub handler: String,
    pub last_modified: String,
    pub memory: i32,
}

#[allow(dead_code)]
pub struct EmrVc {
    pub id: String,
    pub name: String,
    pub state: String,
    pub namespace: String,
    pub jobs: usize,
}

#[allow(dead_code)]
pub struct EmrJobRun {
    pub id: String,
    pub name: String,
    pub state: String,
    pub exit_code: Option<i32>,
    pub logs: Option<String>,
    pub created: String,
}

pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}
