# plan_tui_v2 — TUI rewrite: AWS Rust SDK instead of raw HTTP

## Goal

Replace all raw `reqwest` HTTP calls in `awsem-tui` with the official AWS Rust SDK (`aws-sdk-*`). The TUI will treat the awsem server as a standard AWS endpoint and communicate exclusively through AWS SDK clients.

## Motivation

- Current TUI bypasses AWS API shapes with custom admin JSON endpoints (`/admin/api/s3/buckets`, `/admin/api/cognito/users`, etc.)
- S3 admin endpoints do fragile XML string splitting against RustFS output
- Overview and other admin endpoints leak SQLite internals
- Using the AWS SDK is more robust, standard, and maintainable
- The awsem server already implements correct AWS API shapes for all services

## Architecture

```
TUI (aws-sdk-* clients) → standard AWS API calls → awsem server (:4566) → service handlers → storage
```

Each AWS SDK client points to `http://localhost:4566` with dummy credentials and `us-east-1` region. The server already speaks the correct wire protocol for all services:

| Service | AWS SDK crate | Server API shape |
|---------|--------------|-----------------|
| S3 | `aws-sdk-s3` | REST XML via RustFS proxy |
| Cognito | `aws-sdk-cognitoidentityprovider` | `POST /` with `X-Amz-Target` header |
| Secrets | `aws-sdk-secretsmanager` | `POST /` with `X-Amz-Target` header |
| Lambda | `aws-sdk-lambda` | `/2015-03-31/functions/...` REST |
| EMR | `aws-sdk-emrcontainers` | `/virtualclusters/...` REST |

Non-AWS features (Logs, Overview) will read the log file directly from disk and make individual SDK list calls respectively.

---

## Phase 0 — Revert server-side admin routing changes

**Why**: The admin endpoints (`/admin/api/*`) won't be used by the TUI anymore. Revert my routing changes so the server is back to its original state.

### Files to change

**`crates/awsem-care/src/lib.rs`** — Revert `cfg.route()` back to `web::scope("/admin/api")`:
```rust
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
```

**`crates/awsem-s3/src/lib.rs`** — Revert `cfg.route()` back to `web::scope("/admin/api/s3")`:
```rust
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/api/s3")
            .route("/buckets", web::get().to(admin::list_buckets))
            .route("/objects", web::get().to(admin::list_objects)),
    );
    cfg.route("/{tail:.*}", web::route().to(proxy::s3_handler));
}
```

**`crates/awsem-core/src/defaults.rs`** — Revert `default_log_dir()` to original:
```rust
pub fn default_log_dir() -> Option<String> {
    let base = dirs::config_dir()?;
    Some(base.join("awsem").join("logs").to_string_lossy().into())
}
```

**Note**: The `awsem-care` crate and `awsem-s3/src/admin.rs` files remain in the workspace (they're part of the original codebase). They'll just be unused by the TUI. The `awsem-s3/src/port_forward.rs` drop-fix stays (bugfix, not API change).

---

## Phase 1 — Add AWS SDK dependencies & create module

### Files to change

**`crates/awsem-tui/Cargo.toml`**:
- Remove `reqwest`
- Remove `serde_json` (only needed for result display, keep a minimal usage)
- Keep `serde` for result display types
- Add AWS SDK crates:
  ```toml
  aws-sdk-s3 = "1"
  aws-sdk-cognitoidentityprovider = "1"
  aws-sdk-secretsmanager = "1"
  aws-sdk-lambda = "1"
  aws-sdk-emrcontainers = "1"
  aws-config = "1"
  aws-credential-types = "1"
  ```

**`crates/awsem-tui/src/aws_clients.rs`** (new):
- Create a struct `AwsClients` holding one client per service
- Initialize all clients with:
  - `endpoint_url`: `http://localhost:4566` (from CLI arg)
  - `region`: `us-east-1`
  - `credentials`: dummy access key + secret key (any values work)
  - For S3: `force_path_style: true`
- Provide a helper `check_health()` that tries `ListBuckets` (or any single API call)

**`crates/awsem-tui/src/client.rs`** — **Delete this file** (replaced by `AwsClients`)

**`crates/awsem-tui/src/main.rs`**:
- Replace `Client` with `AwsClients`
- Pass `AwsClients` to `App::new()`
- Health check uses `aws_clients.check_health().await`

---

## Phase 2 — Rewrite App state & refresh

### Files to change

**`crates/awsem-tui/src/app.rs`**:
- Add `aws_clients: AwsClients` field
- Replace all `serde_json::Value` state fields with typed Rust structs:
  - `s3_buckets: Vec<S3Bucket>` (struct with `name: String`)
  - `s3_objects: Vec<S3Object>` (struct with `key: String, size: i64`)
  - `cognito_users: Vec<CognitoUser>` (struct with `username: String, status: String, email: Option<String>`)
  - `secrets: Vec<SecretEntry>` (struct with `name: String, arn: String`)
  - `emr_vcs: Vec<EmrVc>` (struct with `id, name, state: String`)
  - `emr_jobs: Vec<EmrJobRun>` (struct with `id, name, state: String`)
  - `lambda_funcs: Vec<LambdaFn>` (struct with `name, runtime: String, timeout: i64`)
  - `logs: Vec<LogEntry>` (struct with `timestamp, level, target, message: String`)
  - `log_file: String`
- Remove `overview: Option<Value>` (will fetch counts per-service)

**Rewrite `refresh_all()`**:
  - S3: `aws_clients.s3.list_buckets().send().await` → extract bucket names
  - S3 (objects): `aws_clients.s3.list_objects_v2().bucket(&self.s3_bucket).send().await`
  - Cognito: `aws_clients.cognito.list_users().send().await`
  - Secrets: `aws_clients.secrets.list_secrets().send().await`
  - EMR VCs: `aws_clients.emr.list_virtual_clusters().send().await`
  - EMR jobs: `aws_clients.emr.list_job_runs().virtual_cluster_id(&self.emr_vc_id).send().await`
  - Lambda: `aws_clients.lambda.list_functions().send().await`
  - Logs: read `log_file` directly from disk via `std::fs::read_to_string`

**Rewrite `submit_input()`**:
  - CreateBucket: `aws_clients.s3.create_bucket().bucket(&val).send().await`
  - CreateUser (SignUp): `aws_clients.cognito.sign_up().client_id("tui").username(&username).password(&val).send().await`
  - LambdaPayload: `aws_clients.lambda.invoke().function_name(&func).payload(val.into_bytes().into()).send().await`
  - S3UploadKey: `aws_clients.s3.put_object().bucket(&bucket).key(&val).body(content.into()).send().await`
  - EmrSubmit: construct the `StartJobRunInput` struct and call `aws_clients.emr.start_job_run()....`
  - LogFilter: filter logs in-memory from the loaded entries
  - CreateSecret: `aws_clients.secrets.create_secret().name(&name).secret_string(&val).send().await`
  - EditSecret: `aws_clients.secrets.put_secret_value().secret_id(&name).secret_string(&val).send().await`

**Rewrite delete handlers**:
  - Delete bucket: `aws_clients.s3.delete_bucket().bucket(&name).send().await`
  - Delete S3 object: `aws_clients.s3.delete_object().bucket(&bucket).key(&key).send().await`
  - Delete Cognito user: `aws_clients.cognito.admin_delete_user().username(&name).send().await`
  - Delete secret: `aws_clients.secrets.delete_secret().secret_id(&name).send().await`
  - Delete lambda: `aws_clients.lambda.delete_function().function_name(&name).send().await`
  - Delete VC: `aws_clients.emr.delete_virtual_cluster().id(&id).send().await`
  - Cancel job: `aws_clients.emr.cancel_job_run().id(&id).virtual_cluster_id(&vc_id).send().await`

---

## Phase 3 — Rewrite tab renderers (type-safe)

### Files to change

Each `*.rs` render file under `crates/awsem-tui/src/`:
- Replace `serde_json::Value` field access with typed struct field access
- `s3.rs`, `cognito.rs`, `secrets.rs`, `emr.rs`, `lambda.rs`, `logs.rs`

**`crates/awsem-tui/src/overview.rs`**:
- Replace `get_count(data, key)` helper with actual service count calculations
- Overview reads `app.s3_buckets.len()`, `app.cognito_users.len()`, etc.
- Remove `serde_json` dependency from overview

**`crates/awsem-tui/src/logs.rs`**:
- No more HTTP fetch; logs loaded from disk in `refresh_all()`
- Filter in-memory

**`crates/awsem-tui/src/event_loop.rs`**:
- Remove `sel_field()` helper (no more `serde_json::Value` selection)
- Replace with typed index access on `Vec<T>` fields
- Update `list_len()` to use `.len()` on typed Vecs

**`crates/awsem-tui/src/ui.rs`**:
- Remove serde_json dependency (not needed for rendering)
- Update `render_input` and result display to work with `String` types (no JSON conversion)

---

## Phase 4 — Remove unused dependencies & dead code

### Files

- `awsem-tui/Cargo.toml`: Remove `reqwest`, `serde_json`, `serde` (if no longer needed)
- Remove any leftover imports of `serde_json` or `reqwest`
- Verify all `.rs` files in the TUI crate compile without warnings

---

## Phase 5 — Verify & Test

1. `cargo check` — no errors or warnings
2. `cargo clippy` — clean
3. Line count: every `.rs` ≤ 150 lines
4. Manual smoke test:
   - Start awsem server
   - Start TUI
   - Navigate to each tab
   - Verify data loads
   - Test create/delete operations for each service

---

## Risk & Mitigations

| Risk | Mitigation |
|------|-----------|
| `aws-sdk-emrcontainers` might not exist or have different API | Fallback to raw HTTP for EMR only |
| AWS SDK requires HTTPS or special config for HTTP | SDK supports `endpoint_url` + http connector; test in Phase 5 |
| Signature V4 signing might be rejected by server | Server handlers ignore Authorization header; test with dummy creds |
| `aws-sdk-*` crate editions incompatible with Edition 2024 | Edition only affects crate-local syntax, not dependencies |
| Large SDK dependency graph bloats compile time | Acceptable — this is a dev tool, not a library |
| S3 `force_path_style` not supported in older SDK versions | `aws-sdk-s3` 1.x supports `force_path_style` in config |
