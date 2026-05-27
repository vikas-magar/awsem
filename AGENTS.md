# awsem

Local AWS emulator (S3, Cognito, Secrets Manager, EMR on EKS, Lambda).

## Commands

```sh
cargo build              # build all
cargo build -p awsem     # build server binary only
cargo run                # start dev server on :4566
cargo check              # fast type-check (no codegen)
cargo clippy             # lint
cargo fmt                # format
cargo check -p awsem-s3  # single crate check
```

## Architecture

11-crate workspace. Three binaries — `crates/awsem` (server), `crates/awsem-tui` (terminal dashboard), `crates/awsem-spark-agent` (Spark-side helper).

Single actix-web server (`:4566`). Service routing:

| Service | Route | Dispatch |
|---|---|---|
| Cognito | `POST /` | `X-Amz-Target: AWSCognitoIdentityProviderService.*` guard |
| Secrets Manager | `POST /` | `X-Amz-Target: secretsmanager.*` guard |
| S3 | `/{tail:.*}` | catch-all route → RustFS K8s pod |
| Lambda | `/2015-03-31/functions/...` | actix-web path routes (scope) |
| EMR | `/virtualclusters/...`, `/releases` | actix-web path routes |
| Admin | `/admin/api/*` | `awsem-care` crate (overview, logs, user/secret mgmt) |

Cognito / Secrets Manager in-process (rusqlite). S3 proxied to RustFS (K8s Deployment + PVC + port-forward). EMR backed by raw K8s Jobs (spark-submit, RBAC auto-provisioned per VC namespace). Lambda backed by K8s Jobs; payload via temporary K8s Secret mounted at `/var/payload/input` (`_PAYLOAD_FILE` env var).

Event bus (`tokio::sync::broadcast`) wires `_SUCCESS` detection → Lambda trigger.

## Key Constraints

- **Every `.rs` file must be ≤150 lines.** Split or move code when approaching the limit.
- **Edition 2024** — verify crate compatibility before adding dependencies.

## Disable Services

CLI flags: `--no-s3`, `--no-emr`, `--no-lambda`. Omit K8s entirely with `--s3-endpoint http://...` (external S3-compatible endpoint, no RustFS deploy).

## K8s Requirements

K8s cluster expected (Orbstack at `https://127.0.0.1:26443`, Minikube, kind). All resources use namespace `awsem` (configurable). Server auto-deploys RustFS on startup, cleans up on shutdown.

PVC from a prior run may get stuck in `Terminating` — deploy retry loop handles it but can take up to 30 s.

## S3 Access

```sh
AWS_ACCESS_KEY_ID=awsem AWS_SECRET_ACCESS_KEY=awsem aws s3api ...
```

RustFS env vars in K8s Deployment: `RUSTFS_ACCESS_KEY=awsem`, `RUSTFS_SECRET_KEY=awsem`.

## Lambda

Functions run as K8s Jobs. Payload via temporary K8s Secret read by runtime image at `/var/payload/input`. Env vars `_HANDLER` and `_PAYLOAD_FILE` passed to container. `--lambda-runtime-image` required (without it, invoke returns 500). Timeout enforced (function timeout + 10 s grace period). TTL 60 s on completed Jobs.

Event source mappings (S3 bucket ARN → Lambda) auto-trigger on `_SUCCESS` file upload.

## Config

CLI args (`--*`) override TOML at `~/.config/awsem/config.toml`. JWT secret defaults to a generated UUID.
