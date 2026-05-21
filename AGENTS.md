# awsem

Local AWS emulator (S3, Cognito, Secrets Manager, EMR on EKS, Lambda).

## Commands

```sh
cargo build              # build
cargo run                # start dev server on :4566
cargo check              # fast type-check (no codegen)
cargo clippy             # lint
cargo fmt                # format
```

## Architecture

8-crate workspace — binary at `crates/awsem`, service crates as libraries.

Single actix-web server (`:4566`). Request routing:

| Service | Route | Dispatch |
|---|---|---|
| Cognito | `POST /` | `X-Amz-Target` header guard |
| Secrets Manager | `POST /` | `X-Amz-Target` header guard |
| S3 | `/*` | catch-all proxy → RustFS K8s pod |
| Lambda | `/2015-03-31/functions/...` | actix-web path routes |
| EMR | `/virtualclusters/...`, `/releases` | actix-web path routes |

S3 backed by RustFS (K8s Deployment + PVC). Cognito / Secrets Manager
in-process (rusqlite). EMR backed by raw K8s Jobs (spark-submit). Lambda
backed by K8s Jobs with payload via temporary K8s Secret.

Event bus (`tokio::sync::broadcast`) wires `_SUCCESS` detection → Lambda.

## Key Constraints

- **Every `.rs` file must be ≤150 lines.** Split or move code when approaching
  the limit. This is checked after every change.
- **Edition 2024** — verify crate compatibility before adding dependencies.

## K8s Requirements

K8s cluster expected (Orbstack at `https://127.0.0.1:26443`). All resources
use the configured namespace (default: `awsem`). The server auto-deploys
RustFS on startup and cleans up on shutdown.

PVC from a prior run may get stuck in `Terminating` — the deploy retry loop
handles this but can take up to 30 s.

## S3 Access

RustFS credentials: `awsem` / `awsem`:
```sh
AWS_ACCESS_KEY_ID=awsem AWS_SECRET_ACCESS_KEY=awsem aws s3api ...
```

## Lambda

Functions run as K8s Jobs. Payload is passed via a temporary K8s Secret
mounted at `/var/payload/input`. A `--lambda-runtime-image` must be
provided to execute user code; without it, invoke returns a 500.

## Config

CLI args (`--*`) override TOML at `~/.config/awsem/config.toml`. JWT secret
auto-generates as a UUID if not provided.
