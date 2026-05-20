# awsem

Minimal Rust binary project (Rust edition 2024).

## Commands

```sh
cargo build              # build
cargo run                # run the binary
cargo check              # fast type-check without codegen
cargo test               # run tests
cargo clippy             # lint (requires: rustup component add clippy)
cargo fmt                # format (requires: rustup component add rustfmt)
```

## Notes

- Workspace with `crates/awsem` as the sole crate member.
- Edition 2024 — if adding deps, verify they support it.
- No CI, no pre-commit hooks, no task runner configured yet.

## Implementation Plan

See `PLAN.md` for the full step-by-step build plan (1965 lines, 6 phases).
Follow phases in order; each task includes verification commands.
