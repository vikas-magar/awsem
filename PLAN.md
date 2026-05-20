# awsem — Build Plan

## Overview

Local AWS emulator targeting **S3**, **Cognito**, **Secrets Manager**, **EMR on EKS**, and **Lambda**.

- S3 backed by **RustFS** running as a K8s Deployment
- Cognito, Secrets Manager in-process (actix-web + rusqlite)
- EMR backed by **raw K8s Jobs** (spark-submit in cluster mode)
- Lambda backed by **Docker** (bollard)
- Event bus wires `_SUCCESS` file detection → Lambda invocation

## How to Use This Plan

Each phase is ordered and must be completed before the next begins.
Within a phase, tasks are also ordered.
After each task, run the verification command(s) to confirm success.
If a task fails, fix it before moving to the next.

---

## Phase 1: Foundation

### 1.1 Initialize Workspace

Create the Cargo workspace at repo root with all member crates.

**`/Users/anvi/Movies/awsem/Cargo.toml`:**
```toml
[workspace]
resolver = "2"
members = [
    "crates/awsem",
    "crates/awsem-core",
    "crates/awsem-s3",
    "crates/awsem-cognito",
    "crates/awsem-secretsmanager",
    "crates/awsem-emr",
    "crates/awsem-lambda",
    "crates/awsem-events",
]
```

Create each crate with `cargo init --lib` in `crates/awsem-core/`, `crates/awsem-s3/`, etc.
For `crates/awsem/`, use `cargo init --bin`.

Delete the existing `crates/awsem/Cargo.toml` first (it's the old non-workspace one), then recreate.

**`/Users/anvi/Movies/awsem/crates/awsem/Cargo.toml`:**
```toml
[package]
name = "awsem"
version = "0.1.0"
edition = "2024"

[dependencies]
actix-web = "4"
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
awsem-core = { path = "../awsem-core" }
awsem-s3 = { path = "../awsem-s3" }
awsem-cognito = { path = "../awsem-cognito" }
awsem-secretsmanager = { path = "../awsem-secretsmanager" }
awsem-emr = { path = "../awsem-emr" }
awsem-lambda = { path = "../awsem-lambda" }
awsem-events = { path = "../awsem-events" }
```

Set each member crate's `Cargo.toml` with `edition = "2024"` and an `[lib]` section with `name = "awsem_X"` (matching the crate name convention, e.g. `awsem_core`).

**Verification:**
```sh
cargo check 2>&1
# Should succeed with no errors (warnings about dead code are OK)
```

### 1.2 Implement awsem-core

**Dependencies (`crates/awsem-core/Cargo.toml`):**
```toml
[package]
name = "awsem-core"
version = "0.1.0"
edition = "2024"

[lib]
name = "awsem_core"

[dependencies]
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
thiserror = "2"
kube = { version = "0.98", features = ["client", "ws"] }
k8s-openapi = { version = "0.24", features = ["v1_32"] }
futures = "0.3"
tokio-util = "0.7"
base64 = "0.22"
regex = "1"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

**Files to create:**

**`crates/awsem-core/src/lib.rs`**
```rust
pub mod db;
pub mod arn;
pub mod auth;
pub mod service;
pub mod config;
pub mod error;
pub mod k8s;
```

**`crates/awsem-core/src/config.rs`** — CLI config struct with clap:
```rust
#[derive(clap::Parser, Clone)]
pub struct AppConfig {
    #[arg(default_value = "4566")]
    pub port: u16,
    #[arg(long, default_value = "./awsem.db")]
    pub db_path: String,
    #[arg(long)]
    pub kubeconfig: Option<String>,
    #[arg(long, default_value = "awsem")]
    pub k8s_namespace: String,
    #[arg(long)]
    pub rustfs_image: Option<String>,
    #[arg(long, default_value = "10Gi")]
    pub rustfs_pvc_size: String,
    #[arg(long)]
    pub s3_endpoint: Option<String>,   // skip K8s deploy, use this
    #[arg(long, default_value = "false")]
    pub no_s3: bool,
    #[arg(long, default_value = "false")]
    pub no_emr: bool,
    #[arg(long, default_value = "false")]
    pub no_lambda: bool,
    #[arg(long)]
    pub data_dir: Option<String>,       // for Lambda temp code
}
```

**`crates/awsem-core/src/db.rs`** — Database setup:
```rust
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub type DbConn = Arc<Mutex<Connection>>;

pub fn open(path: &str) -> DbConn {
    let conn = Connection::open(path).expect("Failed to open database");
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;").unwrap();
    Arc::new(Mutex::new(conn))
}

pub fn init_schema(conn: &DbConn) {
    let sql = include_str!("../../schema.sql");
    conn.lock().unwrap().execute_batch(sql).expect("Failed to init schema");
}
```

**`crates/awsem-core/src/arn.rs`** — ARN type:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Arn {
    pub partition: String,   // "aws"
    pub service: String,     // "s3", "cognito-idp", etc.
    pub region: String,
    pub account: String,
    pub resource: String,    // "bucket/my-bucket", "userpool/us-east-1_xxx", etc.
}

impl Arn {
    pub fn parse(s: &str) -> Result<Self, String> { /* split on : */ }
    pub fn to_string(&self) -> String { /* format as arn:aws:service:region:account:resource */ }

    pub fn s3_bucket(account: &str, name: &str) -> Self {
        Self { partition: "aws".into(), service: "s3".into(), region: "us-east-1".into(), account: account.into(), resource: format!("bucket/{name}") }
    }
    pub fn cognito_pool(account: &str, region: &str, pool_id: &str) -> Self {
        Self { partition: "aws".into(), service: "cognito-idp".into(), region: region.into(), account: account.into(), resource: format!("userpool/{pool_id}") }
    }
    pub fn secret(account: &str, name: &str) -> Self {
        Self { partition: "aws".into(), service: "secretsmanager".into(), region: "us-east-1".into(), account: account.into(), resource: format!("secret:{name}") }
    }
    pub fn emr_virtual_cluster(account: &str, id: &str) -> Self {
        Self { partition: "aws".into(), service: "emr-containers".into(), region: "us-east-1".into(), account: account.into(), resource: format!("/virtualclusters/{id}") }
    }
    pub fn emr_job_run(account: &str, vc_id: &str, jr_id: &str) -> Self {
        Self { partition: "aws".into(), service: "emr-containers".into(), region: "us-east-1".into(), account: account.into(), resource: format!("/virtualclusters/{vc_id}/jobruns/{jr_id}") }
    }
    pub fn lambda(account: &str, name: &str) -> Self {
        Self { partition: "aws".into(), service: "lambda".into(), region: "us-east-1".into(), account: account.into(), resource: format!("function:{name}") }
    }
}
```

**`crates/awsem-core/src/auth.rs`** — SigV4 header parser:
```rust
#[derive(Debug, Clone)]
pub struct AuthInfo {
    pub access_key: String,   // treated as account ID
    pub region: String,
    pub service: String,
}

pub fn extract_auth_info(headers: &actix_web::HttpRequest) -> AuthInfo {
    // Parse Authorization: AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request
    // Default to test/test/us-east-1 if header missing
    AuthInfo {
        access_key: "000000000000".into(),
        region: "us-east-1".into(),
        service: "s3".into(),
    }
    // For now: always return defaults. SigV4 validation is skipped (dev emulator).
}
```

**`crates/awsem-core/src/service.rs`** — Service trait:
```rust
use actix_web::web;

pub trait AwsService {
    fn name(&self) -> &'static str;
    fn configure(self: Box<Self>, cfg: &mut web::ServiceConfig);
}
```

**`crates/awsem-core/src/error.rs`** — Error type:
```rust
#[derive(Debug, thiserror::Error)]
pub enum AwsemError {
    #[error("Not implemented: {0}")]
    NotImplemented(String),
    #[error("Resource not found: {0}")]
    NotFound(String),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Internal error: {0}")]
    Internal(String),
}
```

**`crates/awsem-core/src/k8s.rs`** — K8s helpers:
```rust
use kube::Client;
use k8s_openapi::api::core::v1::Namespace;

pub async fn try_client() -> Option<Client> {
    match Client::try_default().await {
        Ok(c) => Some(c),
        Err(_) => None,
    }
}

pub async fn ensure_namespace(client: &Client, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let namespaces: kube::api::Api<Namespace> = kube::api::Api::all(client.clone());
    let ns = serde_json::json!({
        "apiVersion": "v1",
        "kind": "Namespace",
        "metadata": { "name": name }
    });
    let ns: Namespace = serde_json::from_value(ns)?;
    match namespaces.create(&kube::api::PostParams::default(), &ns).await {
        Ok(_) => Ok(()),
        Err(kube::Error::Api(e)) if e.code == 409 => Ok(()), // already exists
        Err(e) => Err(e.into()),
    }
}
```

Now also create **`crates/awsem-core/schema.sql`** (the global schema file):

Copy the full SQL schema from the plan's "Database Schema" section (the complete `CREATE TABLE IF NOT EXISTS` statements for all services). Include every table: `awsem_meta`, `s3_notification_configs`, `cognito_user_pools`, `cognito_clients`, `cognito_users`, `secrets_secrets`, `secrets_versions`, `emr_virtual_clusters`, `emr_job_runs`, `lambda_functions`, `lambda_event_source_mappings`.

**Verification:**
```sh
cargo check -p awsem-core 2>&1
```

---

## Phase 2: S3 Proxy + RustFS on K8s

### 2.1 Implement awsem-s3

**`crates/awsem-s3/Cargo.toml`:**
```toml
[package]
name = "awsem-s3"
version = "0.1.0"
edition = "2024"

[lib]
name = "awsem_s3"

[dependencies]
actix-web = "4"
actix-rt = "2"
reqwest = { version = "0.12", features = ["stream"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
anyhow = "1"
awsem-core = { path = "../awsem-core" }
kube = { version = "0.98", features = ["client", "ws"] }
k8s-openapi = { version = "0.24", features = ["v1_32"] }
futures = "0.3"
thiserror = "2"
rusqlite = { version = "0.32", features = ["bundled"] }
```

**`crates/awsem-s3/src/lib.rs`:**
```rust
pub mod proxy;
pub mod rustfs;
pub mod notification;
pub mod event_detect;
```

**`crates/awsem-s3/src/rustfs.rs`** — RustFS lifecycle on K8s:

Implement these public async functions:

```rust
pub struct RustFsConfig {
    pub namespace: String,
    pub image: String,
    pub pvc_size: String,
}

/// Deploy RustFS to K8s: PVC → Deployment → Service.
/// Returns the pod name for port-forwarding.
pub async fn deploy(client: &kube::Client, cfg: &RustFsConfig) -> Result<String>

/// Wait for the RustFS pod to be ready (Running state).
pub async fn wait_ready(client: &kube::Client, namespace: &str, pod_name: &str) -> Result<()>

/// Start a port-forward from a local TCP port to the RustFS pod's port 9000.
/// Returns the local port number.
pub async fn port_forward(client: &kube::Client, namespace: &str, pod_name: &str) -> Result<u16>

/// Delete all RustFS resources (Deployment, Service, PVC).
pub async fn cleanup(client: &kube::Client, cfg: &RustFsConfig) -> Result<()>
```

Implementation details for `deploy`:
1. Create `PersistentVolumeClaim` named `rustfs-data`:
   ```json
   {
     "apiVersion": "v1",
     "kind": "PersistentVolumeClaim",
     "metadata": { "name": "rustfs-data", "namespace": cfg.namespace },
     "spec": { "accessModes": ["ReadWriteOnce"], "resources": { "requests": { "storage": cfg.pvc_size } } }
   }
   ```
2. Create `Deployment` named `rustfs`:
   ```json
   {
     "apiVersion": "apps/v1",
     "kind": "Deployment",
     "metadata": { "name": "rustfs", "namespace": cfg.namespace },
     "spec": {
       "replicas": 1,
       "selector": { "matchLabels": { "app": "rustfs" } },
       "template": {
         "metadata": { "labels": { "app": "rustfs" } },
         "spec": {
           "containers": [{
             "name": "rustfs",
             "image": cfg.image,
             "ports": [{ "containerPort": 9000 }, { "containerPort": 9001 }],
             "env": [
               { "name": "RUSTFS_DATA_DIR", "value": "/data" },
               { "name": "RUSTFS_DEFAULT_REGION", "value": "us-east-1" },
               { "name": "RUSTFS_ROOT_USER", "value": "aw sem" },
               { "name": "RUSTFS_ROOT_PASSWORD", "value": "aw sem" },
             ],
             "volumeMounts": [{ "name": "data", "mountPath": "/data" }]
           }],
           "volumes": [{ "name": "data", "persistentVolumeClaim": { "claimName": "rustfs-data" } }]
         }
       }
     }
   }
   ```
3. Create `Service` named `rustfs-svc`:
   ```json
   {
     "apiVersion": "v1",
     "kind": "Service",
     "metadata": { "name": "rustfs-svc", "namespace": cfg.namespace },
     "spec": {
       "selector": { "app": "rustfs" },
       "ports": [{ "port": 9000, "targetPort": 9000, "name": "s3" }]
     }
   }
   ```

Implementation details for `port_forward`:
1. Use `kube::api::Api::portforward` to create a `Portforwarder`
2. Call `pods.portforward(pod_name, &[9000]).await?`
3. Get `forwarder.take_stream(9000)` 
4. Find a free local TCP port (bind `TcpListener::bind("127.0.0.1:0")`, get local addr, drop)
5. Spawn a tokio task that:
   - `TcpListener::bind(local_addr)` (the free port)
   - For each incoming connection: `tokio::io::copy_bidirectional(client_conn, upstream_conn)`
6. Return the local port number

**`crates/awsem-s3/src/proxy.rs`** — Actix-web S3 proxy handler:

```rust
use actix_web::{web, HttpRequest, HttpResponse, http::StatusCode};
use bytes::Bytes;

pub struct S3State {
    pub rustfs_url: String,            // "http://localhost:PORT"
    pub http_client: reqwest::Client,
    pub db: awsem_core::DbConn,
    pub event_bus: tokio::sync::broadcast::Sender<awsem_events::BusEvent>,
}

pub async fn s3_handler(
    req: HttpRequest,
    body: Bytes,
    state: web::Data<S3State>,
) -> HttpResponse {
    // 1. Build target URL: format!("{}{}", state.rustfs_url, req.uri())
    // 2. Build reqwest request with same method, headers (skip "host"), body
    // 3. Send request
    // 4. Intercept PutObject response for _SUCCESS detection:
    //    - If PUT succeeds AND object key ends with "_SUCCESS":
    //      parse bucket from path, fire event_detect::on_success
    // 5. Return RustFS response with same status, headers, body
}
```

**Route registration in `crates/awsem-s3/src/lib.rs`:**
```rust
pub fn configure(cfg: &mut web::ServiceConfig) {
    // Register a catch-all DefaultService
    cfg.default_service(web::resource("/{tail:.*}").route(web::route().to(proxy::s3_handler)));
}
```

**`crates/awsem-s3/src/notification.rs`** — S3 notification config:

```rust
pub async fn put_notification_config(
    conn: &awsem_core::DbConn,
    bucket: &str,
    config_xml: &str,
) -> Result<()>

pub async fn get_notification_config(
    conn: &awsem_core::DbConn,
    bucket: &str,
) -> Result<Option<String>>

pub fn get_event_targets(
    conn: &awsem_core::DbConn,
    bucket: &str,
    object_key: &str,
) -> Vec<EventTarget>
// Parse stored XML notification config, filter by prefix/suffix rules
```

**`crates/awsem-s3/src/event_detect.rs`**:
```rust
pub async fn on_success(detect: &S3State, bucket: &str, key: &str) {
    if !key.ends_with("_SUCCESS") { return; }
    let targets = notification::get_event_targets(&detect.db, bucket, key);
    for t in targets {
        detect.event_bus.send(awsem_events::BusEvent::S3Notification {
            bucket: bucket.to_string(),
            key: key.to_string(),
            target_arn: t.arn,
            target_type: t.target_type,
        }).ok();
    }
}
```

**Verification:**
```sh
cargo check -p awsem-s3 2>&1
```

### 2.2 Implement awsem-events

**`crates/awsem-events/Cargo.toml`:**
```toml
[package]
name = "awsem-events"
version = "0.1.0"
edition = "2024"

[lib]
name = "awsem_events"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["sync"] }
```

**`crates/awsem-events/src/lib.rs`:**
```rust
#[derive(Clone, Debug)]
pub enum BusEvent {
    S3Notification {
        bucket: String,
        key: String,
        target_arn: String,
        target_type: String,  // "Lambda", "SQS", "SNS"
    },
    LambdaInvocation {
        function_arn: String,
        payload: String,
    },
}

pub fn new_bus(capacity: usize) -> (tokio::sync::broadcast::Sender<BusEvent>, tokio::sync::broadcast::Receiver<BusEvent>) {
    tokio::sync::broadcast::channel(capacity)
}
```

**Verification:**
```sh
cargo check -p awsem-events 2>&1
```

### 2.3 Wire up Binary

Edit **`crates/awsem/src/main.rs`**:

```rust
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = awsem_core::config::AppConfig::parse();
    let db = awsem_core::db::open(&config.db_path);
    awsem_core::db::init_schema(&db);

    // K8s client (optional)
    let kube_client = if !config.no_s3 || !config.no_emr {
        awsem_core::k8s::try_client().await
    } else {
        None
    };

    // Deploy RustFS
    let rustfs_url = if config.no_s3 {
        None
    } else if let Some(endpoint) = &config.s3_endpoint {
        Some(endpoint.clone())  // user-provided external endpoint
    } else if let Some(ref client) = kube_client {
        awsem_core::k8s::ensure_namespace(client, &config.k8s_namespace).await.unwrap();
        let image = config.rustfs_image.clone().unwrap_or_else(|| "rustfs/rustfs:latest".into());
        let rustfs_cfg = awsem_s3::rustfs::RustFsConfig {
            namespace: config.k8s_namespace.clone(),
            image,
            pvc_size: config.rustfs_pvc_size.clone(),
        };
        let pod_name = awsem_s3::rustfs::deploy(client, &rustfs_cfg).await.unwrap();
        awsem_s3::rustfs::wait_ready(client, &config.k8s_namespace, &pod_name).await.unwrap();
        let local_port = awsem_s3::rustfs::port_forward(client, &config.k8s_namespace, &pod_name).await.unwrap();
        Some(format!("http://localhost:{}", local_port))
    } else {
        tracing::warn!("No K8s client available and --s3-endpoint not set. S3 disabled.");
        None
    };

    // Event bus
    let (event_tx, _event_rx) = awsem_events::new_bus(1024);

    // HTTP client for S3 proxy
    let http_client = reqwest::Client::new();

    // Build actix-web server
    let rustfs_url = rustfs_url.clone();
    let db_clone = db.clone();
    let http_client_clone = http_client.clone();
    let event_tx_clone = event_tx.clone();

    let server = actix_web::HttpServer::new(move || {
        use actix_web::web;

        let mut app = actix_web::App::new();

        // Shared state for S3
        if let Some(ref url) = rustfs_url {
            let s3_state = awsem_s3::S3State {
                rustfs_url: url.clone(),
                http_client: http_client_clone.clone(),
                db: db_clone.clone(),
                event_bus: event_tx_clone.clone(),
            };
            app = app.app_data(web::Data::new(s3_state));
            app = app.configure(awsem_s3::configure);
        }

        // Placeholder: Cognito, Secrets, EMR, Lambda will be added in later phases
        // app = app.configure(awsem_cognito::configure);
        // app = app.configure(awsem_secretsmanager::configure);
        // app = app.configure(awsem_emr::configure);
        // app = app.configure(awsem_lambda::configure);

        app
    })
    .bind(("0.0.0.0", config.port))?
    .run();

    tracing::info!("awsem listening on {}", config.port);
    server.await
}
```

**Verification:**
```sh
cargo check 2>&1
# cargo build (will take longer)
# Then test with a running K8s cluster:
# cargo run -- --s3-endpoint http://localhost:9000
# (requires a RustFS instance accessible at that URL)
# or if a K8s cluster is available:
# cargo run
```

---

## Phase 3: Cognito User Pools

### 3.1 Implement Store

**`crates/awsem-cognito/Cargo.toml`:**
```toml
[package]
name = "awsem-cognito"
version = "0.1.0"
edition = "2024"

[lib]
name = "awsem_cognito"

[dependencies]
actix-web = "4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_xml_rs = "0.6"
tracing = "0.1"
anyhow = "1"
argon2 = "0.5"
jsonwebtoken = "9"
rand = "0.8"
uuid = { version = "1", features = ["v4"] }
time = "0.3"
rusqlite = { version = "0.32", features = ["bundled"] }
awsem-core = { path = "../awsem-core" }
```

**`crates/awsem-cognito/src/lib.rs`:**
```rust
pub mod handlers;
pub mod models;
pub mod store;
pub mod tokens;
```

**`crates/awsem-cognito/src/models.rs`** — Data types matching AWS Cognito API:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserPool {
    pub id: String,              // "us-east-1_xxxxxxxxx"
    pub name: String,
    pub arn: String,
    pub config: serde_json::Value,  // full CreateUserPool input JSON
    pub created_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserPoolClient {
    pub id: String,
    pub pool_id: String,
    pub client_name: String,
    pub client_id: String,       // random 26-char alphanumeric
    pub client_secret: String,   // random 64-char
    pub config: serde_json::Value,
    pub created_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: String,
    pub pool_id: String,
    pub username: String,
    pub password_hash: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub status: String,           // UNCONFIRMED, CONFIRMED, FORCE_CHANGE_PASSWORD
    pub attributes: serde_json::Value,
    pub created_at: i64,
}
```

**`crates/awsem-cognito/src/store.rs`** — SQLite CRUD operations:

```rust
pub fn create_pool(conn: &awsem_core::DbConn, name: &str, config: &serde_json::Value) -> UserPool
pub fn get_pool(conn: &awsem_core::DbConn, pool_id: &str) -> Option<UserPool>
pub fn update_pool(conn: &awsem_core::DbConn, pool_id: &str, config: &serde_json::Value) -> bool
pub fn delete_pool(conn: &awsem_core::DbConn, pool_id: &str) -> bool
pub fn list_pools(conn: &awsem_core::DbConn, max_results: i32, next_token: Option<&str>) -> (Vec<UserPool>, Option<String>)

pub fn create_client(conn: &awsem_core::DbConn, pool_id: &str, client_name: &str, config: &serde_json::Value) -> UserPoolClient
pub fn get_client(conn: &awsem_core::DbConn, client_id: &str) -> Option<UserPoolClient>
pub fn update_client(conn: &awsem_core::DbConn, client_id: &str, config: &serde_json::Value) -> bool
pub fn delete_client(conn: &awsem_core::DbConn, client_id: &str) -> bool

pub fn create_user(conn: &awsem_core::DbConn, pool_id: &str, username: &str, password: &str, email: Option<&str>, phone: Option<&str>, attributes: &serde_json::Value) -> User
pub fn get_user(conn: &awsem_core::DbConn, pool_id: &str, username: &str) -> Option<User>
pub fn get_user_by_id(conn: &awsem_core::DbConn, user_id: &str) -> Option<User>
pub fn update_user_password(conn: &awsem_core::DbConn, pool_id: &str, username: &str, new_password: &str) -> bool
pub fn update_user_status(conn: &awsem_core::DbConn, user_id: &str, status: &str) -> bool
pub fn list_users(conn: &awsem_core::DbConn, pool_id: &str) -> Vec<User>
```

Each function executes SQL against the rusqlite connection. Use `conn.lock().unwrap()` pattern.

Pool ID format: `"us-east-1_" + random_9chars` (alphanumeric, lowercase).
Client ID: random 26-char alphanumeric.
Client Secret: random 64-char (hex).

Password hashing: use `argon2::Argon2::default()` with `password_hash::SaltString::generate(&mut OsRng)`.

**`crates/awsem-cognito/src/tokens.rs`** — Mock JWT generation:

```rust
pub fn get_or_create_key_pair(conn: &awsem_core::DbConn) -> (Vec<u8>, Vec<u8>) {
    // Check awsem_meta table for "cognito_jwt_private_key"
    // If missing, generate a new RSA 2048-bit key pair
    // Store both PEM-encoded keys in awsem_meta
    // Return (private_key_pem, public_key_pem)
}

pub fn generate_access_token(pool_id: &str, client_id: &str, username: &str, user_sub: &str, private_key: &[u8]) -> String
pub fn generate_id_token(pool_id: &str, client_id: &str, username: &str, email: &str, private_key: &[u8]) -> String
pub fn generate_refresh_token() -> String
```

JWT claims structure for AccessToken:
```json
{
  "sub": "user_sub",
  "iss": "arn:aws:cognito-idp:us-east-1:000000000000:userpool/pool_id",
  "client_id": "client_id",
  "token_use": "access",
  "username": "username",
  "scope": "aws.cognito.signin.user.admin",
  "auth_time": 1234567890,
  "exp": 1234567890 + 3600,
  "iat": 1234567890
}
```

Use `jsonwebtoken::Header::new(Algorithm::RS256)` and `encode(&header, &claims, &EncodingKey::from_rsa_pem(private_key)?)`.

**`crates/awsem-cognito/src/handlers.rs`** — Actix-web request handlers:

Protocol detection: `X-Amz-Target` header starts with `AWSCognitoIdentityProviderService.{Operation}`.
All requests are POST with JSON body.

Create a single handler that reads `X-Amz-Target` and dispatches:

```rust
pub async fn cognito_handler(
    req: HttpRequest,
    body: Bytes,
    db: web::Data<awsem_core::DbConn>,
) -> HttpResponse {
    let target = req.headers().get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let op = target.strip_prefix("AWSCognitoIdentityProviderService.").unwrap_or("");

    let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::json!({}));

    match op {
        "CreateUserPool" => handle_create_pool(&db, &json),
        "DescribeUserPool" => handle_describe_pool(&db, &json),
        "UpdateUserPool" => handle_update_pool(&db, &json),
        "DeleteUserPool" => handle_delete_pool(&db, &json),
        "ListUserPools" => handle_list_pools(&db, &json),
        "CreateUserPoolClient" => handle_create_client(&db, &json),
        "DescribeUserPoolClient" => handle_describe_client(&db, &json),
        "UpdateUserPoolClient" => handle_update_client(&db, &json),
        "DeleteUserPoolClient" => handle_delete_client(&db, &json),
        "SignUp" => handle_sign_up(&db, &json),
        "InitiateAuth" => handle_initiate_auth(&db, &json),
        "AdminCreateUser" => handle_admin_create_user(&db, &json),
        "AdminGetUser" => handle_admin_get_user(&db, &json),
        "AdminSetUserPassword" => handle_admin_set_user_password(&db, &json),
        "ForgotPassword" => handle_forgot_password(&db, &json),
        "ConfirmForgotPassword" => handle_confirm_forgot_password(&db, &json),
        _ => {
            HttpResponse::build(StatusCode::BAD_REQUEST)
                .json(serde_json::json!({
                    "__type": "UnknownOperationException",
                    "message": format!("Unknown operation: {}", op)
                }))
        }
    }
}
```

Each handler:
1. Extracts parameters from JSON input
2. Calls store functions
3. Builds JSON response matching AWS format
4. Returns `HttpResponse::Ok().json(response_body)` for success, or appropriate error status

**Response format examples:**

CreateUserPool success:
```json
{
  "UserPool": {
    "Id": "us-east-1_abc123def",
    "Name": "my-pool",
    "Arn": "arn:aws:cognito-idp:us-east-1:000000000000:userpool/us-east-1_abc123def",
    "CreationDate": 1234567890.0,
    "LastModifiedDate": 1234567890.0
  }
}
```

InitiateAuth success:
```json
{
  "AuthenticationResult": {
    "AccessToken": "eyJ...",
    "IdToken": "eyJ...",
    "RefreshToken": "abc...",
    "TokenType": "Bearer",
    "ExpiresIn": 3600
  },
  "ChallengeName": null
}
```

Error format:
```json
{
  "__type": "InvalidParameterException",
  "message": "User pool does not exist"
}
```

Register the handler in lib.rs:
```rust
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/", web::post().to(handlers::cognito_handler));
}
```

**Verification:**
```sh
cargo check -p awsem-cognito 2>&1
```

### 3.2 Register Cognito in Binary

In `crates/awsem/src/main.rs`, uncomment the Cognito configure line:
```rust
app = app.configure(awsem_cognito::configure);
```

Make sure the Cognito handler does NOT conflict with the S3 catch-all. Since Cognito receives `X-Amz-Target` header and S3 receives different requests, the routing can be:

1. First register Cognito (specific route `"/"` with POST method)
2. Then register S3 as default (catch-all)

Actix-web will try Cognito's route first (it's explicit), and if the request doesn't match (e.g., it's a PUT to /bucket, not a POST to / with X-Amz-Target header), it falls through to the DefaultService (S3 proxy).

Actually, the better approach: put Cognito handlers at a more specific path. But Cognito's API sends all requests to `POST /`. So we need to differentiate by header:

In the main binary, use an actix-web middleware or `web::resource` with a guard:
```rust
use actix_web::guard::Guard;
struct CognitoGuard;
impl Guard for CognitoGuard {
    fn check(&self, req: &actix_web::RequestHead) -> bool {
        req.headers().get("x-amz-target")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.starts_with("AWSCognitoIdentityProviderService"))
            .unwrap_or(false)
    }
}

// In HttpServer::new:
app.service(
    web::resource("/")
        .guard(CognitoGuard)
        .route(web::post().to(awsem_cognito::handlers::cognito_handler))
);
```

But for simplicity, just check the header in the handler. The S3 proxy will forward requests that aren't for Cognito.

**Verification:**
```sh
cargo check 2>&1
# Then run and test:
# cargo run -- --s3-endpoint http://localhost:9000 &
# aws --endpoint-url http://localhost:4566 cognito-idp create-user-pool --pool-name test
```

---

## Phase 4: Secrets Manager

### 4.1 Implement Store

**`crates/awsem-secretsmanager/Cargo.toml`:**
```toml
[package]
name = "awsem-secretsmanager"
version = "0.1.0"
edition = "2024"

[lib]
name = "awsem_secretsmanager"

[dependencies]
actix-web = "4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
anyhow = "1"
uuid = { version = "1", features = ["v4"] }
rand = "0.8"
chrono = "0.4"
rusqlite = { version = "0.32", features = ["bundled"] }
awsem-core = { path = "../awsem-core" }
```

**`crates/awsem-secretsmanager/src/lib.rs`**:
```rust
pub mod handlers;
pub mod models;
pub mod store;
```

**`crates/awsem-secretsmanager/src/models.rs`:**
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Secret {
    pub id: String,
    pub name: String,
    pub arn: String,
    pub description: Option<String>,
    pub kms_key_id: Option<String>,
    pub tags: Vec<Tag>,
    pub created_at: i64,
    pub last_changed: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecretVersion {
    pub id: String,
    pub secret_id: String,
    pub version_id: String,    // UUID v4
    pub value: String,
    pub staging_labels: Vec<String>,  // ["AWSCURRENT"], ["AWSPREVIOUS"]
    pub created_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Tag {
    pub key: String,
    pub value: String,
}
```

**`crates/awsem-secretsmanager/src/store.rs`** — CRUD operations:

```rust
pub fn create_secret(conn: &awsem_core::DbConn, name: &str, value: &str, description: Option<&str>, tags: &[Tag]) -> (Secret, SecretVersion)
pub fn get_secret(conn: &awsem_core::DbConn, name_or_id: &str) -> Option<Secret>
pub fn get_secret_value(conn: &awsem_core::DbConn, secret_id: &str, version_id: Option<&str>, stage: Option<&str>) -> Option<SecretVersion>
pub fn put_secret_value(conn: &awsem_core::DbConn, secret_id: &str, value: &str) -> Option<(Secret, SecretVersion)>
pub fn update_secret(conn: &awsem_core::DbConn, secret_id: &str, description: Option<&str>, tags: Option<&[Tag]>) -> bool
pub fn delete_secret(conn: &awsem_core::DbConn, secret_id: &str, recovery_window: i64) -> bool
pub fn restore_secret(conn: &awsem_core::DbConn, secret_id: &str) -> bool
pub fn list_secrets(conn: &awsem_core::DbConn) -> Vec<Secret>
```

Staging label logic for `put_secret_value`:
1. Get current AWSCURRENT version → move its label to AWSPREVIOUS (remove AWSCURRENT, add AWSPREVIOUS)
2. Create new version with AWSCURRENT label
3. Return new version

**`crates/awsem-secretsmanager/src/handlers.rs`** — X-Amz-Target handler:

Same pattern as Cognito. Protocol: `X-Amz-Target: secretsmanager.{Operation}`.
All POST with JSON body.

Operations to implement:
- `CreateSecret` → / (POST)
- `GetSecretValue` → / (POST)
- `PutSecretValue` → / (POST)
- `UpdateSecret` → / (POST)
- `DeleteSecret` → / (POST)
- `DescribeSecret` → / (POST)
- `ListSecrets` → / (POST)
- `RestoreSecret` → / (POST)

Note: `BatchGetSecretValue` is NOT in scope for initial implementation.

**Response format for CreateSecret:**
```json
{
  "ARN": "arn:aws:secretsmanager:us-east-1:000000000000:secret:my-secret-abc123",
  "Name": "my-secret",
  "VersionId": "uuid-v4-here"
}
```

Register a guard similar to Cognito for `X-Amz-Target` starting with `secretsmanager.`.

**`crates/awsem-secretsmanager/src/lib.rs`:**
```rust
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/", web::post().to(handlers::secrets_handler));
}
```

### 4.2 Register in Binary

In `main.rs`, add `app = app.configure(awsem_secretsmanager::configure);`.

Update the guards so S3 fallback doesn't swallow Secrets/Cognito requests:
- Register service-specific routes with guards BEFORE the DefaultService
- Actix-web checks guarded routes first, then falls to DefaultService

**Verification:**
```sh
cargo check 2>&1
# cargo run -- --s3-endpoint http://localhost:9000 &
# aws --endpoint-url http://localhost:4566 secretsmanager create-secret --name test --secret-string "hello"
```

---

## Phase 5: EMR on EKS

### 5.1 Implement awsem-emr

**`crates/awsem-emr/Cargo.toml`:**
```toml
[package]
name = "awsem-emr"
version = "0.1.0"
edition = "2024"

[lib]
name = "awsem_emr"

[dependencies]
actix-web = "4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
anyhow = "1"
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"
rusqlite = { version = "0.32", features = ["bundled"] }
awsem-core = { path = "../awsem-core" }
kube = { version = "0.98", features = ["client", "ws"] }
k8s-openapi = { version = "0.24", features = ["v1_32"] }
futures = "0.3"
tokio = { version = "1", features = ["full"] }
```

**`crates/awsem-emr/src/lib.rs`:**
```rust
pub mod handlers;
pub mod models;
pub mod store;
pub mod k8s_job;
pub mod spark_params;
pub mod release_images;
```

**`crates/awsem-emr/src/models.rs`:**
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VirtualCluster {
    pub id: String,
    pub name: String,
    pub arn: String,
    pub namespace: String,
    pub eks_cluster: String,
    pub state: String,          // RUNNING, TERMINATED
    pub created_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JobRun {
    pub id: String,
    pub name: String,
    pub arn: String,
    pub virtual_cluster_id: String,
    pub job_driver: serde_json::Value,  // StartJobRun's jobDriver
    pub state: String,                  // PENDING, SUBMITTED, RUNNING, SUCCESS, FAILED, CANCELLING, CANCELLED
    pub kubernetes_job_name: Option<String>,
    pub output_prefix: Option<String>,
    pub created_at: i64,
    pub finished_at: Option<i64>,
    pub failure_reason: Option<String>,
}
```

**`crates/awsem-emr/src/store.rs`** — SQLite CRUD:

```rust
pub fn create_virtual_cluster(conn: &awsem_core::DbConn, name: &str, namespace: &str, eks_cluster: &str) -> VirtualCluster
pub fn get_virtual_cluster(conn: &awsem_core::DbConn, vc_id: &str) -> Option<VirtualCluster>
pub fn list_virtual_clusters(conn: &awsem_core::DbConn, states: Option<&[String]>) -> Vec<VirtualCluster>
pub fn delete_virtual_cluster(conn: &awsem_core::DbConn, vc_id: &str) -> bool

pub fn create_job_run(conn: &awsem_core::DbConn, name: &str, vc_id: &str, driver: &serde_json::Value) -> JobRun
pub fn get_job_run(conn: &awsem_core::DbConn, jr_id: &str) -> Option<JobRun>
pub fn list_job_runs(conn: &awsem_core::DbConn, vc_id: &str) -> Vec<JobRun>
pub fn update_job_run_state(conn: &awsem_core::DbConn, jr_id: &str, state: &str, finished_at: Option<i64>, failure_reason: Option<&str>) -> bool
pub fn update_job_run_k8s_name(conn: &awsem_core::DbConn, jr_id: &str, k8s_name: &str) -> bool
pub fn get_job_run_by_k8s_name(conn: &awsem_core::DbConn, k8s_job_name: &str) -> Option<JobRun>
```

**`crates/awsem-emr/src/release_images.rs`**:
```rust
pub fn resolve_spark_image(release_label: &str) -> String {
    // Simple mapping of EMR release labels to Spark images
    let label = release_label.to_lowercase();
    if label.contains("emr-7") || label.contains("emr-6.15") {
        "spark:3.5.3".to_string()
    } else if label.contains("emr-6.14") || label.contains("emr-6.13") {
        "spark:3.5.0".to_string()
    } else if label.contains("emr-6.12") || label.contains("emr-6.11") || label.contains("emr-6.10") {
        "spark:3.4.0".to_string()
    } else {
        "spark:3.5.3".to_string()  // default fallback
    }
}
```

**`crates/awsem-emr/src/spark_params.rs`** — Parse spark-submit parameters:

```rust
pub fn parse(input: &str) -> Vec<String> {
    // Simple tokenizer that splits on whitespace
    // respecting single and double quotes
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for c in input.chars() {
        if escaped {
            current.push(c);
            escaped = false;
            continue;
        }
        if c == '\\' { escaped = true; continue; }
        if c == '\'' && !in_double { in_single = !in_single; continue; }
        if c == '"' && !in_single { in_double = !in_double; continue; }
        if c.is_whitespace() && !in_single && !in_double {
            if !current.is_empty() {
                args.push(current.clone());
                current.clear();
            }
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}
```

**`crates/awsem-emr/src/k8s_job.rs`** — K8s Job creation:

```rust
pub async fn submit_job_run(
    client: &kube::Client,
    namespace: &str,
    vc_id: &str,
    job_run: &models::JobRun,
    params: &serde_json::Value,
) -> Result<String> {
    // Extract sparkSubmitJobDriver or sparkSqlJobDriver from params
    let entry_point = params["sparkSubmitJobDriver"]["entryPoint"].as_str()
        .or_else(|| params["sparkSqlJobDriver"]["entryPoint"].as_str())
        .unwrap_or("");
    let entry_point_args: Vec<String> = params["sparkSubmitJobDriver"]["entryPointArguments"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let spark_params_str = params["sparkSubmitJobDriver"]["sparkSubmitParameters"]
        .as_str()
        .unwrap_or("");
    let spark_sql_params = params["sparkSqlJobDriver"]["sparkSqlParameters"]
        .as_str()
        .unwrap_or("");

    // Parse spark-submit parameters
    let spark_args = spark_params::parse(spark_params_str);

    // Get Spark image
    let release_label = params.get("releaseLabel").and_then(|v| v.as_str()).unwrap_or("emr-6.15.0");
    let spark_image = release_images::resolve_spark_image(release_label);

    // Build container args
    let mut container_args = vec!["spark-submit".to_string()];

    // Add spark-submit parameters from the parsed string
    // Also inject S3 endpoint so Spark can reach RustFS
    let mut extra_conf = vec![
        format!("spark.hadoop.fs.s3a.endpoint=http://rustfs-svc:9000"),
        format!("spark.hadoop.fs.s3a.access.key=awsem"),
        format!("spark.hadoop.fs.s3a.secret.key=awsem"),
        format!("spark.hadoop.fs.s3a.path.style.access=true"),
        format!("spark.kubernetes.namespace={}", namespace),
    ];

    // Build args vector: first spark-submit, then confs, then spark args, then entry point
    let mut all_args = vec!["spark-submit".to_string()];

    // Add spark-submit parameters (the raw parsed ones from the user)
    let mut i = 0;
    while i < spark_args.len() {
        all_args.push(spark_args[i].clone());
        i += 1;
    }

    // Add S3 endpoint config
    for conf in &extra_conf {
        all_args.push("--conf".to_string());
        all_args.push(conf.clone());
    }

    // Add entry point and arguments
    all_args.push(entry_point.to_string());
    all_args.extend(entry_point_args);

    // For Spark SQL jobs
    if !spark_sql_params.is_empty() {
        all_args.push("--sql".to_string());
        all_args.push(spark_sql_params.to_string());
    }

    // Build K8s Job object
    let job_name = format!("awsem-{}-{}", &job_run.virtual_cluster_id[..8], &job_run.id[..8]);
    // Sanitize for DNS-1123: lowercase alphanumeric + hyphens only
    let job_name: String = job_name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' }).collect();
    let job_name = job_name.trim_matches('-').to_string();

    let job = k8s_openapi::api::batch::v1::Job {
        metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
            name: Some(job_name.clone()),
            namespace: Some(namespace.to_string()),
            labels: Some(std::collections::BTreeMap::from([
                ("app.kubernetes.io/managed-by".into(), "awsem".into()),
                ("awsem/virtual-cluster-id".into(), vc_id.to_string()),
                ("awsem/job-run-id".into(), job_run.id.clone()),
            ])),
            ..Default::default()
        },
        spec: Some(k8s_openapi::api::batch::v1::JobSpec {
            backoff_limit: Some(params["retryPolicyConfiguration"]["maxAttempts"].as_i64().unwrap_or(1) as i32),
            ttl_seconds_after_finished: Some(3600),
            template: k8s_openapi::api::core::v1::PodTemplateSpec {
                spec: Some(k8s_openapi::api::core::v1::PodSpec {
                    restart_policy: Some("Never".into()),
                    containers: vec![k8s_openapi::api::core::v1::Container {
                        name: "spark-submit".into(),
                        image: Some(spark_image),
                        args: Some(all_args),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        }),
    };

    let jobs_api: kube::api::Api<k8s_openapi::api::batch::v1::Job> =
        kube::api::Api::namespaced(client.clone(), namespace);
    jobs_api.create(&kube::api::PostParams::default(), &job).await?;

    Ok(job_name)
}
```

**`crates/awsem-emr/src/handlers.rs`** — REST-JSON handlers:

EMR Containers API uses REST-JSON paths, not just X-Amz-Target.

Routes:
- `POST /virtualclusters` → create_virtual_cluster
- `GET /virtualclusters` → list_virtual_clusters
- `GET /virtualclusters/{vc_id}` → describe_virtual_cluster
- `DELETE /virtualclusters/{vc_id}` → delete_virtual_cluster
- `POST /virtualclusters/{vc_id}/jobruns` → start_job_run
- `GET /virtualclusters/{vc_id}/jobruns` → list_job_runs
- `GET /virtualclusters/{vc_id}/jobruns/{jr_id}` → describe_job_run
- `DELETE /virtualclusters/{vc_id}/jobruns/{jr_id}` → cancel_job_run

```rust
pub async fn handle_create_virtual_cluster(
    body: web::Json<serde_json::Value>,
    db: web::Data<awsem_core::DbConn>,
    app_config: web::Data<awsem_core::config::AppConfig>,
) -> HttpResponse {
    // 1. Extract name, containerProvider from body
    // 2. Create namespace in K8s if available
    // 3. Store in SQLite via store::create_virtual_cluster
    // 4. Return { id, arn, name }
}

pub async fn handle_start_job_run(
    path: web::Path<String>,  // vc_id
    body: web::Json<serde_json::Value>,
    db: web::Data<awsem_core::DbConn>,
    kube: web::Data<Option<kube::Client>>,
    app_config: web::Data<awsem_core::config::AppConfig>,
) -> HttpResponse {
    // 1. Look up virtual cluster
    // 2. Create job run in SQLite (state = SUBMITTED)
    // 3. If K8s client available:
    //    a. Get namespace from virtual cluster
    //    b. Call k8s_job::submit_job_run
    //    c. Update job run with K8s job name
    //    d. Set state = RUNNING
    // 4. Return { id, name, arn, virtualClusterId }
}
```

**Job completion watcher** (background task):

```rust
pub async fn start_job_watcher(
    client: kube::Client,
    namespace: String,
    db: awsem_core::DbConn,
    event_bus: tokio::sync::broadcast::Sender<awsem_events::BusEvent>,
) {
    let jobs_api: kube::api::Api<k8s_openapi::api::batch::v1::Job> =
        kube::api::Api::namespaced(client.clone(), &namespace);
    let watcher = kube::runtime::watcher(jobs_api, kube::runtime::watcher::Config::default()
        .labels("app.kubernetes.io/managed-by=awsem"));
    let mut stream = kube::runtime::watcher::WatchStreamExt::boxed(watcher.into_stream());

    while let Some(event) = stream.try_next().await.unwrap_or(None) {
        if let kube::runtime::watcher::Event::Apply(job) = event {
            let status = job.status.as_ref();
            let conditions = status.and_then(|s| s.conditions.as_ref());

            let succeeded = conditions
                .and_then(|c| c.iter().find(|c| c.type_ == "Complete" && c.status == "True"));
            let failed = conditions
                .and_then(|c| c.iter().find(|c| c.type_ == "Failed" && c.status == "True"));

            let job_run_id = job.metadata.labels
                .and_then(|l| l.get("awsem/job-run-id"))
                .cloned();

            if let Some(jr_id) = job_run_id {
                if succeeded.is_some() {
                    // Job completed successfully
                    store::update_job_run_state(&db, &jr_id, "SUCCESS", Some(chrono::Utc::now().timestamp()), None);

                    // Check for _SUCCESS file
                    // Extract output path from the job run params (stored as job_driver_json)
                    // Call S3 head-object via reqwest to check _SUCCESS
                    // If found, fire event
                    check_success_file(&db, &jr_id, &event_bus).await;
                } else if failed.is_some() {
                    let reason = failed.unwrap().message.clone().unwrap_or_default();
                    store::update_job_run_state(&db, &jr_id, "FAILED", Some(chrono::Utc::now().timestamp()), Some(&reason));
                }
            }
        }
    }
}

async fn check_success_file(
    db: &awsem_core::DbConn,
    job_run_id: &str,
    event_bus: &tokio::sync::broadcast::Sender<awsem_events::BusEvent>,
) {
    // 1. Get job run from DB
    // 2. Extract output prefix from job_driver_json (if available)
    // 3. If output prefix set, check _SUCCESS via S3 API
    // 4. If found, publish BusEvent::S3Notification
}
```

**`crates/awsem-emr/src/lib.rs`:**
```rust
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg
        .route("/virtualclusters", web::post().to(handlers::handle_create_virtual_cluster))
        .route("/virtualclusters", web::get().to(handlers::handle_list_virtual_clusters))
        .route("/virtualclusters/{vc_id}", web::get().to(handlers::handle_describe_virtual_cluster))
        .route("/virtualclusters/{vc_id}", web::delete().to(handlers::handle_delete_virtual_cluster))
        .route("/virtualclusters/{vc_id}/jobruns", web::post().to(handlers::handle_start_job_run))
        .route("/virtualclusters/{vc_id}/jobruns", web::get().to(handlers::handle_list_job_runs))
        .route("/virtualclusters/{vc_id}/jobruns/{jr_id}", web::get().to(handlers::handle_describe_job_run))
        .route("/virtualclusters/{vc_id}/jobruns/{jr_id}", web::delete().to(handlers::handle_cancel_job_run));
}

// Start the background watcher when EMR is initialized
pub async fn init_watcher(
    client: kube::Client,
    namespace: String,
    db: awsem_core::DbConn,
    event_bus: tokio::sync::broadcast::Sender<awsem_events::BusEvent>,
) {
    tokio::spawn(async move {
        start_job_watcher(client, namespace, db, event_bus).await;
    });
}
```

### 5.2 Register in Binary

In `main.rs`, add EMR configuration and start the job watcher:

```rust
if let Some(ref client) = kube_client {
    app = app.configure(|cfg| awsem_emr::configure(cfg));
    awsem_emr::init_watcher(
        client.clone(),
        config.k8s_namespace.clone(),
        db.clone(),
        event_tx.clone(),
    ).await;
}
```

**Verification:**
```sh
cargo check 2>&1
```

---

## Phase 6: Lambda

### 6.1 Implement awsem-lambda

**`crates/awsem-lambda/Cargo.toml`:**
```toml
[package]
name = "awsem-lambda"
version = "0.1.0"
edition = "2024"

[lib]
name = "awsem_lambda"

[dependencies]
actix-web = "4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
anyhow = "1"
uuid = { version = "1", features = ["v4"] }
rusqlite = { version = "0.32", features = ["bundled"] }
awsem-core = { path = "../awsem-core" }
awsem-events = { path = "../awsem-events" }
bollard = "0.18"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
futures = "0.3"
lambda_events = "0.8"
```

**`crates/awsem-lambda/src/lib.rs`:**
```rust
pub mod handlers;
pub mod models;
pub mod store;
pub mod runtime;
```

**`crates/awsem-lambda/src/models.rs`:**
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LambdaFunction {
    pub name: String,
    pub arn: String,
    pub runtime: String,         // "python3.12", "nodejs20.x", "provided.al2", etc.
    pub handler: String,         // "index.handler"
    pub role: String,
    pub timeout: i32,            // seconds, default 3
    pub memory_size: i32,        // MB, default 128
    pub code_zip: Vec<u8>,       // ZIP file containing function code
    pub created_at: i64,
    pub last_modified: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventSourceMapping {
    pub id: String,
    pub function_arn: String,
    pub event_source_arn: String,
    pub enabled: bool,
    pub batch_size: i32,
}
```

**`crates/awsem-lambda/src/store.rs`:**
```rust
pub fn create_function(conn: &awsem_core::DbConn, name: &str, runtime: &str, handler: &str, role: &str, code_zip: &[u8], timeout: i32, memory_size: i32) -> LambdaFunction
pub fn get_function(conn: &awsem_core::DbConn, name: &str) -> Option<LambdaFunction>
pub fn get_function_by_arn(conn: &awsem_core::DbConn, arn: &str) -> Option<LambdaFunction>
pub fn delete_function(conn: &awsem_core::DbConn, name: &str) -> bool
pub fn list_functions(conn: &awsem_core::DbConn) -> Vec<LambdaFunction>

pub fn create_event_source_mapping(conn: &awsem_core::DbConn, function_arn: &str, event_source_arn: &str, batch_size: i32) -> EventSourceMapping
pub fn list_event_source_mappings(conn: &awsem_core::DbConn, function_arn: Option<&str>) -> Vec<EventSourceMapping>
pub fn delete_event_source_mapping(conn: &awsem_core::DbConn, mapping_id: &str) -> bool
```

**`crates/awsem-lambda/src/runtime.rs`** — Docker-backed execution:

```rust
/// Invoke a Lambda function by extracting its code and running it in Docker.
/// Returns the function's response as a JSON string.
pub async fn invoke(
    docker: &bollard::Docker,
    function: &models::LambdaFunction,
    event_payload: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // 1. Create a temp directory
    // 2. Extract the ZIP into it
    // 3. Determine the Lambda runtime container image
    let image = runtime_image(&function.runtime);

    // 4. Create container:
    //    Using the AWS Lambda Runtime Interface Emulator (RIE) approach:
    //    - Image: public.ecr.aws/lambda/{runtime}
    //    - Cmd: [function.handler]
    //    - Env vars: AWS_LAMBDA_FUNCTION_NAME, AWS_LAMBDA_FUNCTION_VERSION, _HANDLER, etc.
    //    - Bind mount: temp_dir → /var/task
    // 5. Start container
    // 6. POST event to http://localhost:{port}/2015-03-31/functions/function/invocations
    // 7. Read response
    // 8. Stop and remove container
    // 9. Clean up temp dir
    // 10. Return response

    // For simplicity in first implementation:
    // - Use bollard to run `python3 -c "import sys; sys.path.insert(0, '/var/task'); exec(open('/var/task/{handler}').read())"`
    //   or similar for each runtime
    // - This avoids needing the full RIE

    // SIMPLER FIRST ATTEMPT — just return a mock response:
    tracing::info!("Lambda invoke: {} with {} bytes payload", function.name, event_payload.len());
    Ok(serde_json::json!({
        "statusCode": 200,
        "body": "Hello from awsem Lambda"
    }).to_string())
}

fn runtime_image(runtime: &str) -> &str {
    match runtime {
        "python3.13" | "python3.12" | "python3.11" => "public.ecr.aws/lambda/python:3.12",
        "nodejs22.x" | "nodejs20.x" | "nodejs18.x" => "public.ecr.aws/lambda/nodejs:20",
        "java21" | "java17" | "java11" => "public.ecr.aws/lambda/java:21",
        "provided.al2" | "provided.al2023" => "public.ecr.aws/lambda/provided:al2",
        "rust.al2" | "rust.al2023" => "public.ecr.aws/lambda/provided:al2",
        _ => "public.ecr.aws/lambda/python:3.12",
    }
}
```

**`crates/awsem-lambda/src/handlers.rs`** — REST-JSON handlers:

Routes:
- `POST /2015-03-31/functions` → create_function
- `GET /2015-03-31/functions` → list_functions
- `GET /2015-03-31/functions/{name}` → get_function
- `DELETE /2015-03-31/functions/{name}` → delete_function
- `POST /2015-03-31/functions/{name}/invocations` → invoke_function
- `POST /2021-10-31/functions/{name}/invocations` → invoke_function (async)
- `POST /2015-03-31/event-source-mappings` → create_event_source_mapping
- `GET /2015-03-31/event-source-mappings` → list_event_source_mappings
- `DELETE /2015-03-31/event-source-mappings/{id}` → delete_event_source_mapping

```rust
pub async fn handle_invoke_function(
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
    db: web::Data<awsem_core::DbConn>,
    docker: web::Data<Option<bollard::Docker>>,
) -> HttpResponse {
    let function_name = path.into_inner();
    let function = match store::get_function(&db, &function_name) {
        Some(f) => f,
        None => return HttpResponse::build(StatusCode::NOT_FOUND)
            .json(serde_json::json!({"message": "Function not found"})),
    };

    let payload = serde_json::to_string(&body.into_inner()).unwrap_or_default();

    // Try Docker execution, fall back to mock
    let response = if let Some(ref docker) = *docker {
        match runtime::invoke(docker, &function, &payload).await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Lambda Docker execution failed: {}, returning mock", e);
                mock_response(&function)
            }
        }
    } else {
        mock_response(&function)
    };

    HttpResponse::Ok()
        .content_type("application/json")
        .body(response)
}

fn mock_response(function: &models::LambdaFunction) -> String {
    serde_json::json!({
        "statusCode": 200,
        "body": format!("Hello from {} (mock)", function.name),
        "executedVersion": "$LATEST"
    }).to_string()
}
```

**`crates/awsem-lambda/src/lib.rs`:**
```rust
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg
        .route("/2015-03-31/functions", web::post().to(handlers::handle_create_function))
        .route("/2015-03-31/functions", web::get().to(handlers::handle_list_functions))
        .route("/2015-03-31/functions/{name}", web::get().to(handlers::handle_get_function))
        .route("/2015-03-31/functions/{name}", web::delete().to(handlers::handle_delete_function))
        .route("/2015-03-31/functions/{name}/invocations", web::post().to(handlers::handle_invoke_function))
        .route("/2015-03-31/event-source-mappings", web::post().to(handlers::handle_create_event_source_mapping))
        .route("/2015-03-31/event-source-mappings", web::get().to(handlers::handle_list_event_source_mappings))
        .route("/2015-03-31/event-source-mappings/{id}", web::delete().to(handlers::handle_delete_event_source_mapping));
}
```

### 6.2 Wire Event Bus → Lambda

In `main.rs`, spawn an event consumer task:

```rust
// After creating event bus
let event_rx_clone = event_tx.subscribe();
let db_clone = db.clone();
let docker = bollard_client.clone();
tokio::spawn(async move {
    let mut rx = event_rx_clone;
    while let Ok(event) = rx.recv().await {
        match event {
            awsem_events::BusEvent::S3Notification { bucket, key, target_arn, target_type } => {
                if target_type == "Lambda" {
                    // Extract function name from ARN
                    let function_name = target_arn.rsplit(':').next().unwrap_or(&target_arn);
                    let function_name = function_name.strip_prefix("function:").unwrap_or(function_name);

                    // Build S3 event payload
                    let s3_event = serde_json::json!({
                        "Records": [{
                            "eventVersion": "2.1",
                            "eventSource": "aws:s3",
                            "eventName": "ObjectCreated:Put",
                            "s3": {
                                "bucket": { "name": bucket, "arn": format!("arn:aws:s3:::{}", bucket) },
                                "object": { "key": key, "size": 0, "eTag": "d41d8cd98f00b204e9800998ecf8427e" }
                            }
                        }]
                    });

                    // Look up function and invoke
                    if let Some(func) = awsem_lambda::store::get_function(&db_clone, &function_name) {
                        let payload = serde_json::to_string(&s3_event).unwrap_or_default();
                        if let Some(ref d) = docker {
                            let _ = awsem_lambda::runtime::invoke(d, &func, &payload).await;
                        }
                    }
                }
            }
            _ => {}
        }
    }
});
```

### 6.3 Register in Binary

```rust
// main.rs: detect Docker socket
let bollard_client = match bollard::Docker::connect_with_local_defaults() {
    Ok(d) => {
        tracing::info!("Docker socket available — Lambda execution enabled");
        Some(d)
    }
    Err(_) => {
        tracing::warn!("Docker socket not available — Lambda will return mock responses");
        None
    }
};
```

**Verification:**
```sh
cargo check 2>&1
```

---

## Full Binary Wiring (`crates/awsem/src/main.rs`)

The complete main.rs should:

1. Parse config (clap)
2. Init tracing
3. Open SQLite, init schema
4. Try K8s client
5. Manage RustFS lifecycle (deploy/port-forward or use existing endpoint)
6. Init Docker client
7. Create event bus
8. Build HttpServer:
   - Register Cognito (with X-Amz-Target guard)
   - Register Secrets Manager (with X-Amz-Target guard)
   - Register EMR (REST-JSON routes)
   - Register Lambda (REST routes)
   - Register S3 as DefaultService (catch-all)
9. Start EMR job watcher background task
10. Start event bus consumer background task
11. Start server
12. On shutdown: cleanup RustFS

---

## Verification Commands

After each phase, run these to verify:

### Phase 2 (S3)
```sh
# Requires a K8s cluster with RustFS or --s3-endpoint
aws --endpoint-url http://localhost:4566 s3 mb s3://test-bucket
aws --endpoint-url http://localhost:4566 s3 cp ./test.txt s3://test-bucket/
aws --endpoint-url http://localhost:4566 s3 ls s3://test-bucket/
aws --endpoint-url http://localhost:4566 s3 rm s3://test-bucket/test.txt
aws --endpoint-url http://localhost:4566 s3 rb s3://test-bucket
```

### Phase 3 (Cognito)
```sh
aws --endpoint-url http://localhost:4566 cognito-idp create-user-pool --pool-name test-pool
aws --endpoint-url http://localhost:4566 cognito-idp list-user-pools --max-results 10
```

### Phase 4 (Secrets Manager)
```sh
aws --endpoint-url http://localhost:4566 secretsmanager create-secret --name test-secret --secret-string "hello world"
aws --endpoint-url http://localhost:4566 secretsmanager get-secret-value --secret-id test-secret
aws --endpoint-url http://localhost:4566 secretsmanager list-secrets
aws --endpoint-url http://localhost:4566 secretsmanager delete-secret --secret-id test-secret
```

### Phase 5 (EMR)
```sh
# Requires K8s cluster
aws --endpoint-url http://localhost:4566 emr-containers create-virtual-cluster \
  --name test-vc \
  --container-provider '{"id": "test-eks", "type": "EKS", "info": {"eksInfo": {"namespace": "test-ns"}}}'
aws --endpoint-url http://localhost:4566 emr-containers list-virtual-clusters
```

### Phase 6 (Lambda)
```sh
# Create a simple Python Lambda
cat > handler.py << 'EOF'
def handler(event, context):
    return {"statusCode": 200, "body": "Hello from Lambda!"}
EOF
zip function.zip handler.py

aws --endpoint-url http://localhost:4566 lambda create-function \
  --function-name test-func \
  --runtime python3.12 \
  --handler handler.handler \
  --role arn:aws:iam::000000000000:role/lambda-role \
  --zip-file fileb://function.zip

aws --endpoint-url http://localhost:4566 lambda invoke \
  --function-name test-func \
  --payload '{"key": "value"}' \
  response.json
```

---

## Complete Database Schema

Copy this into `crates/awsem-core/schema.sql`:

```sql
CREATE TABLE IF NOT EXISTS awsem_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS s3_notification_configs (
    bucket_name TEXT PRIMARY KEY,
    config_xml TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS cognito_user_pools (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    arn TEXT NOT NULL UNIQUE,
    config_json TEXT NOT NULL DEFAULT '{}',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS cognito_clients (
    id TEXT PRIMARY KEY,
    pool_id TEXT NOT NULL REFERENCES cognito_user_pools(id),
    client_name TEXT NOT NULL,
    client_id TEXT NOT NULL UNIQUE,
    client_secret TEXT NOT NULL,
    config_json TEXT NOT NULL DEFAULT '{}',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS cognito_users (
    id TEXT PRIMARY KEY,
    pool_id TEXT NOT NULL REFERENCES cognito_user_pools(id),
    username TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    email TEXT,
    phone TEXT,
    status TEXT NOT NULL DEFAULT 'UNCONFIRMED',
    attributes_json TEXT NOT NULL DEFAULT '{}',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(pool_id, username)
);

CREATE TABLE IF NOT EXISTS secrets_secrets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    arn TEXT NOT NULL UNIQUE,
    description TEXT,
    kms_key_id TEXT,
    tags_json TEXT DEFAULT '[]',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_changed TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP
);

CREATE TABLE IF NOT EXISTS secrets_versions (
    id TEXT PRIMARY KEY,
    secret_id TEXT NOT NULL REFERENCES secrets_secrets(id),
    version_id TEXT NOT NULL,
    value TEXT NOT NULL,
    staging_labels_json TEXT NOT NULL DEFAULT '[]',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS emr_virtual_clusters (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    arn TEXT NOT NULL UNIQUE,
    namespace TEXT NOT NULL,
    eks_cluster TEXT NOT NULL DEFAULT '',
    state TEXT NOT NULL DEFAULT 'RUNNING',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS emr_job_runs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    arn TEXT NOT NULL UNIQUE,
    virtual_cluster_id TEXT NOT NULL REFERENCES emr_virtual_clusters(id),
    job_driver_json TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'PENDING',
    kubernetes_job_name TEXT,
    output_prefix TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    finished_at TIMESTAMP
);

CREATE TABLE IF NOT EXISTS lambda_functions (
    name TEXT PRIMARY KEY,
    arn TEXT NOT NULL UNIQUE,
    runtime TEXT NOT NULL,
    handler TEXT NOT NULL,
    code_zip BLOB,
    role TEXT NOT NULL,
    timeout INTEGER DEFAULT 3,
    memory_size INTEGER DEFAULT 128,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_modified TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS lambda_event_source_mappings (
    id TEXT PRIMARY KEY,
    function_arn TEXT NOT NULL REFERENCES lambda_functions(arn),
    event_source_arn TEXT NOT NULL,
    enabled INTEGER DEFAULT 1,
    batch_size INTEGER DEFAULT 10,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

---

## Edge Cases Summary

Refer to these when implementing:

1. **K8s unavailable**: S3, Cognito, Secrets work. EMR returns descriptive errors. `--no-s3 --no-emr` for pure local mode.
2. **Port-forward drops**: Wrap port-forward in a retry loop (exponential backoff: 1s, 2s, 4s, 8s… max 60s).
3. **Cognito user exists**: Check UNIQUE constraint on `(pool_id, username)`. Return `UsernameExistsException`.
4. **Secret not found**: Return `ResourceNotFoundException` with `__type` field.
5. **EMR VC not found**: Return 404 with JSON body `{"message": "Virtual cluster not found"}`.
6. **Spark job fails**: K8s Job condition `Failed=true` → update state to FAILED.
7. **No _SUCCESS file after completion**: Retry 3 times (5s/15s/45s). If still missing, mark SUCCESS anyway.
8. **Lambda code too large for SQLite BLOB**: Store in filesystem `{data-dir}/lambda/{name}.zip`, store path in DB.
9. **Concurrent requests**: `rusqlite::Connection` wrapped in `Arc<Mutex<>>`. For read-heavy workloads, consider a connection pool in future iterations.
10. **Actix-web route conflicts**: Use guards for X-Amz-Target services. Register them BEFORE the S3 DefaultService.
