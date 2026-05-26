# awsem — Usage Guide

Local AWS emulator (S3, Cognito, Secrets Manager, EMR on EKS, Lambda) on port 4566.

## Requirements

- **Rust** (edition 2024) — `rustup target add ...` not needed
- **K8s cluster** (OrbStack, Minikube, kind, Docker Desktop) — for S3 (RustFS), EMR, Lambda
- **kubectl** with a current context
- **AWS CLI v2** configured with any credentials (SigV4 is proxied, not validated)

## Installation

```sh
cargo build --release
./target/release/awsem
```

## Configuration

Config is read from `~/.config/awsem/config.toml` (auto-created). CLI flags override file values.

### CLI flags

| Flag | Default | Description |
|------|---------|-------------|
| `--port` | `4566` | HTTP listen port |
| `--config-file` | `~/.config/awsem/config.toml` | Path to TOML config |
| `--db-path` | `~/.config/awsem/data/awsem.db` | SQLite database path |
| `--kubeconfig` | `~/.kube/config` | K8s config path |
| `--k8s-namespace` | `awsem` | K8s namespace for deployments |
| `--rustfs-image` | `rustfs/rustfs:latest` | RustFS container image |
| `--rustfs-pvc-size` | `10Gi` | RustFS PVC size |
| `--s3-endpoint` | auto (RustFS via port-forward) | External S3 endpoint (skip K8s deploy) |
| `--no-s3` | `false` | Disable S3 service |
| `--no-emr` | `false` | Disable EMR service |
| `--no-lambda` | `false` | Disable Lambda service |
| `--lambda-runtime-image` | — | Default Lambda runtime image |
| `--data-dir` | `~/.config/awsem/data` | Data directory |
| `--log-dir` | `~/.config/awsem/logs` | Log directory (daily rotation) |
| `--log-level` | `info` | Tracing log level |

### Config file (`~/.config/awsem/config.toml`)

```toml
port = 4566
k8s_namespace = "awsem"
rustfs_image = "rustfs/rustfs:latest"
rustfs_pvc_size = "10Gi"
log_level = "info"
```

## Services

### S3 (RustFS proxy)

Deploys RustFS as a K8s Deployment, port-forwards, and proxies all requests transparently.

```sh
aws s3 mb s3://my-bucket --endpoint-url http://localhost:4566
aws s3 cp file.txt s3://my-bucket/ --endpoint-url http://localhost:4566
aws s3 ls s3://my-bucket --endpoint-url http://localhost:4566
```

SigV4 works because the Host header is passed through unmodified.

### Cognito

```sh
aws cognito-idp sign-up \
  --client-id test \
  --username user@example.com \
  --password P@ssw0rd \
  --endpoint-url http://localhost:4566

aws cognito-idp initiate-auth \
  --auth-flow USER_PASSWORD_AUTH \
  --auth-parameters USERNAME=user@example.com,PASSWORD=P@ssw0rd \
  --endpoint-url http://localhost:4566

aws cognito-idp global-sign-out \
  --access-token <token> \
  --endpoint-url http://localhost:4566

aws cognito-idp get-user \
  --access-token <token> \
  --endpoint-url http://localhost:4566
```

### Secrets Manager

```sh
aws secretsmanager create-secret \
  --name my-secret \
  --secret-string "hello" \
  --endpoint-url http://localhost:4566

aws secretsmanager get-secret-value \
  --secret-id my-secret \
  --endpoint-url http://localhost:4566

aws secretsmanager put-secret-value \
  --secret-id my-secret \
  --secret-string "updated" \
  --endpoint-url http://localhost:4566

aws secretsmanager describe-secret \
  --secret-id my-secret \
  --endpoint-url http://localhost:4566

aws secretsmanager list-secrets \
  --endpoint-url http://localhost:4566

aws secretsmanager delete-secret \
  --secret-id my-secret \
  --endpoint-url http://localhost:4566

aws secretsmanager restore-secret \
  --secret-id my-secret \
  --endpoint-url http://localhost:4566
```

### EMR on EKS

```sh
aws emr-containers create-virtual-cluster \
  --name my-cluster \
  --container-provider '{"id":"test","type":"EKS","info":{"eksInfo":{"namespace":"spark-jobs"}}}' \
  --endpoint-url http://localhost:4566

aws emr-containers list-virtual-clusters \
  --endpoint-url http://localhost:4566

aws emr-containers delete-virtual-cluster \
  --id <vc-id> \
  --endpoint-url http://localhost:4566

aws emr-containers start-job-run \
  --virtual-cluster-id <vc-id> \
  --name pi-job \
  --release-label emr-7.1.0-latest \
  --job-driver '{"sparkSubmitJobDriver":{"entryPoint":"local:///opt/spark/examples/src/main/python/pi.py","sparkSubmitParameters":"--num-executors 2"}}' \
  --endpoint-url http://localhost:4566
```

Job runs execute via K8s Jobs (spark-submit in cluster mode). RBAC (ServiceAccount + Role + RoleBinding) is provisioned automatically per virtual cluster namespace. Driver pods are tracked via label `awsem-job-run-id`.

### Lambda

```sh
# Create function with a pre-baked custom runtime image
aws lambda create-function \
  --function-name my-func \
  --runtime provided.al2023 \
  --handler handler.handler \
  --role arn:aws:iam::000000000000:role/lambda-role \
  --image-uri my-registry/my-lambda-image:latest \
  --endpoint-url http://localhost:4566

# Invoke synchronously
aws lambda invoke \
  --function-name my-func \
  --payload '{"key":"value"}' \
  --endpoint-url http://localhost:4566 \
  output.json

# Invoke async (Event)
aws lambda invoke \
  --function-name my-func \
  --invocation-type Event \
  --payload '{}' \
  --endpoint-url http://localhost:4566 \
  /dev/null
```

Lambda runs as a K8s Job. The custom runtime image reads `_HANDLER` and `_PAYLOAD` env vars and writes the response to stdout. Pod completion is detected via `kube::runtime::watcher`.

#### Event source mappings (S3 → Lambda trigger)

```sh
aws lambda create-event-source-mapping \
  --function-name my-func \
  --event-source-arn arn:aws:s3:::my-bucket \
  --endpoint-url http://localhost:4566

aws lambda list-event-source-mappings \
  --endpoint-url http://localhost:4566
```

When a `_SUCCESS` file is uploaded to the mapped S3 bucket, the function is invoked automatically with the bucket/key in the payload.

## Logging

- Daily rotating logs: `~/.config/awsem/logs/awsem.YYYY-MM-DD.log`
- Stdout with ANSI colors
- `RUST_LOG` env var overrides the configured log level

## Component Architecture

```
awsem (port 4566)
├── S3 (RustFS K8s proxy — catch-all route)
├── Cognito (in-process SQLite + JWT)
├── Secrets Manager (in-process SQLite)
├── EMR on EKS (K8s Jobs — spark-submit cluster mode)
│   └── Job watcher (polls driver pods every 15s, _SUCCESS fallback)
└── Lambda (K8s Jobs)
    ├── Event source mapping trigger (S3 _SUCCESS → Lambda)
    └── Watcher-based pod completion detection
```
