# awsem — Capabilities & AWS Gaps

## S3 (via RustFS proxy)

### Supported
- All S3 operations proxied transparently to RustFS (GET, PUT, DELETE, LIST, HEAD, etc.)
- SigV4 passthrough (Host header preserved, signature not validated)
- Bucket creation/deletion/file upload/download/list via `aws s3` CLI
- `_SUCCESS` file detection for Lambda triggers

### Not supported / gaps
- No S3 API validation (errors bubble from RustFS)
- No bucket policy enforcement
- No encryption at rest configuration
- No versioning
- No lifecycle rules
- No CORS configuration
- No website hosting
- No notifications (SNS/SQS/Lambda) — only S3 → Lambda via event source mappings and `_SUCCESS` polling
- No multipart upload tracking
- K8s dependency: RustFS must be deployed and available

## Cognito

### Supported operations (X-Amz-Target)
- `SignUp` — creates user with argon2-hashed password
- `InitiateAuth` (USER_PASSWORD_AUTH) — returns access/ID/refresh JWTs
- `RespondToAuthChallenge` — returns mock `NEW_PASSWORD_REQUIRED` response
- `GlobalSignOut` — no-op (returns empty 200)
- `GetUser` — validates access token, returns username + sub

### Not supported / gaps
- Only one user pool (`us-east-1_default`), auto-created
- No `AdminCreateUser`, `AdminInitiateAuth`, `AdminSetUserPassword`
- No `ConfirmSignUp`, `ResendConfirmationCode`
- No `ForgotPassword`, `ConfirmForgotPassword`
- No `ChangePassword`, `UpdateUserAttributes`, `VerifyUserAttribute`
- No `ListUsers`, `DescribeUserPool`, `DescribeUserPoolClient`
- No SRP (Secure Remote Password) protocol
- No refresh token rotation (tokens are stateless JWTs)
- No Lambda triggers (pre-signup, post-auth, etc.)
- No MFA, no device tracking
- No user pool domain / hosted UI
- JWT secret is hardcoded (`"awsem-dev-secret"`)
- No client credentials validation (any `client-id` accepted)

## Secrets Manager

### Supported operations (X-Amz-Target)
- `CreateSecret` — name + SecretString + description + KmsKeyId
- `GetSecretValue` — by name or ARN
- `PutSecretValue` — new version on existing secret
- `UpdateSecret` — update description/KmsKeyId
- `DeleteSecret` — soft-delete (sets `deleted_at`)
- `DescribeSecret` — full metadata
- `ListSecrets` — all non-deleted secrets
- `RestoreSecret` — un-delete

### Not supported / gaps
- No `SecretBinary` (only `SecretString`)
- No `RotationEnabled` / rotation scheduling
- No `SecretList` pagination (`NextToken` always null)
- No automatic deletion scheduling (window parameter ignored)
- No KMS integration (KmsKeyId stored but unused)
- No resource policies
- No tags-based filtering
- No replication (regional)

## EMR on EKS

### Supported operations
- `CreateVirtualCluster` — with name + K8s namespace
- `ListVirtualClusters` — all clusters ordered by creation date
- `DeleteVirtualCluster` — soft-delete (sets `TERMINATED`)
- `StartJobRun` — spark-submit in **cluster mode** as K8s Job
  - Automatic `--master k8s://...` injection
  - User `--master`/`--deploy-mode` stripped from sparkSubmitParameters
  - RBAC provisioning: Spark ServiceAccount + Role (pods/services/configmaps/PVCs) + RoleBinding per VC namespace
  - Label `awsem-job-run-id=<job_id>` on driver pod for tracking
  - Spark image: `apache/spark:latest` for release labels starting with `emr-`
- Job watcher polls driver pods every 15s:
  - K8s pod status (primary)
  - `_SUCCESS` file in S3 (fallback)

### Not supported / gaps
- No `ListJobRuns`, `DescribeJobRun`, `CancelJobRun`
- No `TagResource`, `UntagResource`
- EMR release labels mapped to `apache/spark:latest` (no real EMR images)
- No monitoring dashboard or managed endpoint
- No auto-scaling
- No interactive endpoints (Spark History Server, Livy)
- No security configuration support
- RBAC is basic (one-size-fits-all rules)
- Job watcher runs on fixed 15s interval (not event-driven)
- No Spark driver/executor resource customization beyond `sparkSubmitParameters`

## Lambda

### Supported operations
- `CreateFunction` — name, runtime, handler, role, image, timeout, memory, code zip
- `GetFunction` — by name (returns Configuration)
- `ListFunctions` — all functions ordered by last modified
- `DeleteFunction` — removes DB record + extracted code directory
- `Invoke` — synchronous (RequestResponse) and async (Event)
  - K8s Job execution with `_HANDLER` and `_PAYLOAD` env vars
  - Pod logs captured as response body
  - Watcher-based completion detection (immediate, not polling)
  - Timeout enforcement (function timeout + 10s grace period)
  - Auto-cleanup of completed Jobs (TTL: 60s)
- Event source mappings (CRUD):
  - S3 bucket ARN → Lambda function mapping
  - Auto-trigger on `_SUCCESS` file upload via event bus

### Not supported / gaps
- No `UpdateFunctionConfiguration` / `UpdateFunctionCode`
- No `PublishVersion`, `UpdateAlias`, `ListAliases`, `ListVersionsByFunction`
- No `GetFunctionConfiguration` (use `GetFunction`)
- No `InvokeWithResponseStream` (URL invocation)
- No Lambda Layers
- No VPC configuration
- No environment variables (beyond `_HANDLER`/`_PAYLOAD`)
- No DLQ / `DeadLetterConfig`
- No reserved concurrency / provisioned concurrency
- No `FunctionUrl` (Lambda function URLs)
- Code zip is extracted but ignored during invocation (image-based execution only)
- `Event` invocation sends to event bus (async), not guaranteed delivery
- No tracing (X-Ray)
- No file system mounts (EFS)
- K8s dependency: no offline mode beyond mock responses

## General Limitations

- **Single port (4566)** — all services share one HTTP listener with routing by path + X-Amz-Target header
- **No SigV4 validation** — requests are proxied as-is; RustFS (S3) handles its own auth
- **No AWS SDK compatibility testing** — tested with AWS CLI v2 only
- **No TLS** — HTTP only
- **No multi-region** — everything is `us-east-1`
- **No IAM** — no policies, no roles (Lambda `Role` field is stored but unused)
- **No CloudWatch** — no metrics, no logs group
- **No CloudFormation/Terraform** — no infrastructure-as-code support
- **Single-process** — no horizontal scaling
- **SQLite** — no concurrent write scaling; database is the single state source for Cognito, Secrets, EMR, Lambda
- **K8s required** for S3, EMR, Lambda — not suitable for pure local development without a cluster
