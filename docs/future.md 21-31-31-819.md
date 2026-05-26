# awsem — Future Improvements

## Short-term (next phases)

### S3
- [ ] Multipart upload support (track uploads database table, proxy back to RustFS)
- [ ] S3 notification configuration persistence and SNS/SQS publishing
- [ ] Bucket listing pagination

### Cognito
- [ ] Admin API surface (`AdminCreateUser`, `AdminInitiateAuth`, `AdminSetUserPassword`, `ListUsers`)
- [ ] Confirmation flow (`ConfirmSignUp`)
- [ ] Password recovery (`ForgotPassword`, `ConfirmForgotPassword`)
- [ ] User attribute management (`UpdateUserAttributes`, `GetUser` with full attributes)
- [ ] Configurable JWT signing key (not hardcoded)
- [ ] Configurable user pool (support multiple pools via pool ID in request)
- [ ] Client credentials validation

### Secrets Manager
- [ ] `ListSecrets` pagination (`NextToken` support)
- [ ] `SecretBinary` support (base64-encoded)
- [ ] `GetRandomPassword`
- [ ] `CancelRotateSecret` / `RotateSecret` (stub)
- [ ] Automatic deletion scheduling with recovery window
- [ ] Tags-based filtering in `ListSecrets`

### EMR on EKS
- [ ] `ListJobRuns`, `DescribeJobRun`, `CancelJobRun` API endpoints
- [ ] `TagResource`/`UntagResource`
- [ ] Real EMR release images (not just `apache/spark:latest`)
- [ ] Spark executor resource customization through virtual cluster config
- [ ] Event-driven job watcher (Watcher-based instead of polling)

### Lambda
- [ ] `UpdateFunctionConfiguration` and `UpdateFunctionCode`
- [ ] `PublishVersion` and aliases (`CreateAlias`, `UpdateAlias`)
- [ ] `InvokeWithResponseStream` (response streaming)
- [ ] Lambda environment variables support
- [ ] Lambda Layers support
- [ ] VPC configuration stub
- [ ] Reserved/concurrency settings
- [ ] Function URL support

## Medium-term

### New services
- [ ] **SQS** (Simple Queue Service) — in-memory/SQLite-backed queue with K8s Jobs for consumers
- [ ] **SNS** (Simple Notification Service) — topic CRUD, publish, subscription to SQS/Lambda
- [ ] **DynamoDB** — local SQLite-backed document store with partition/sort key querying
- [ ] **SES** (Simple Email Service) — email sending via SMTP capture or file dump
- [ ] **CloudWatch Logs** — log group/stream CRUD, put-log-events endpoint
- [ ] **IAM** — user/role/policy CRUD, STS `AssumeRole`

### Integration
- [ ] Event bus → SQS queue for async Lambda triggers
- [ ] S3 notifications → SNS topics → SQS queues → Lambda
- [ ] CloudWatch Logs subscription filter → Lambda

### Developer experience
- [ ] Health endpoint (`/_awsem/health`)
- [ ] Reset endpoint (`/_awsem/reset`) to clear all state
- [ ] Admin UI or dashboard (read-only state browser)
- [ ] OpenAPI/Smithy spec generation from handlers
- [ ] Docker Compose deployment (one-shot without K8s dependency)
- [ ] Helm chart for K8s deployment

### Operations
- [ ] Configurable CORS for admin endpoints
- [ ] Metrics endpoint (Prometheus format)
- [ ] Structured JSON logging option
- [ ] Graceful shutdown with drain timeout
- [ ] Configuration reload on SIGHUP
- [ ] Database backup/restore commands

### Testing
- [ ] Integration test suite using `aws` CLI against all service endpoints
- [ ] K8s-less test mode (mocked kube client)
- [ ] Property-based testing for request parsing
- [ ] Benchmark suite for Lambda invocation latency

## Long-term

### Architecture
- [ ] Split into separate processes per service (microservices)
- [ ] gRPC inter-service communication
- [ ] Horizontal scaling with shared database (PostgreSQL instead of SQLite)
- [ ] WebSocket-based event streaming for real-time job/function logs
- [ ] Plugin system for custom service implementations
- [ ] Multi-region simulation (latency injection, region-scoped state)

### Service completeness
- [ ] **EMR Studio** integration — interactive notebook support via K8s
- [ ] **EMR Serverless** compatibility layer
- [ ] **Lambda SnapStart** support
- [ ] **Cognito** — full OAuth 2.0 / OIDC compliance (authorization code grant, PKCE)
- [ ] **Secrets Manager** — automatic rotation with custom Lambda rotation function
- [ ] **Full S3 compatibility** — pass the AWS S3 compatibility test suite
- [ ] **DynamoDB** — transactions, streams, TTL, global tables

### Ecosystem
- [ ] Terraform provider for testing infrastructure-as-code against awsem
- [ ] AWS SDK integration test harness (auto-discover and test against local endpoints)
- [ ] VS Code extension for local state browsing
- [ ] CI/CD pipeline that runs integration tests against awsem before AWS deployment
