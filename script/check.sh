#!/bin/sh
set -eu
ROOT=$(CDPATH= cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

EXPECTED_RUST="1.97.1"
ACTUAL_RUST=$(rustc --version | awk '{print $2}')
if [ "$ACTUAL_RUST" != "$EXPECTED_RUST" ]; then
  echo "error: expected rustc $EXPECTED_RUST, found $ACTUAL_RUST" >&2
  exit 1
fi

if [ ! -f Cargo.lock ]; then
  echo "error: Cargo.lock is missing; run ./script/refresh-lock.sh and review it first" >&2
  exit 1
fi

./script/architecture-check.sh
cargo fmt --all --check
cargo test --locked --workspace --all-features --all-targets
cargo test --locked -p configlab --doc
cargo clippy --locked --workspace --all-features --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --all-features --no-deps

# Feature-isolation checks catch cfg mistakes that an all-features build hides.
cargo check --locked -p configlab --no-default-features
cargo check --locked -p configlab --no-default-features --features derive
cargo check --locked -p configlab --no-default-features --features toml
cargo check --locked -p configlab --no-default-features --features json
cargo check --locked -p configlab --no-default-features --features yaml
cargo check --locked -p configlab --no-default-features --features derive,toml
cargo check --locked -p configlab --no-default-features --features derive,json
cargo check --locked -p configlab --no-default-features --features derive,yaml
cargo check --locked -p configlab --no-default-features --features toml,json
cargo check --locked -p configlab --no-default-features --features toml,yaml
cargo check --locked -p configlab --no-default-features --features json,yaml

# Exercise optimized compilation as a guard against release-profile-only issues.
cargo build --locked -p configlab --all-features --release

# Public examples are actual workspace consumers, not privileged in-crate demos.
cargo run --locked --quiet -p example-basic-app
cargo run --locked --quiet -p example-cli-tool -- --jobs 8 --no-minify
cargo run --locked --quiet -p example-contextual-service -- --port 9090
cargo run --locked --quiet -p example-runtime-service
cargo run --locked --quiet -p example-platform-integration

# Packaging must work independently of workspace-only path resolution.
./script/package-check.sh

# Record a release-mode timing baseline without making timing a flaky pass/fail threshold.
./script/performance-baseline.sh

# Release metadata and public identity must remain internally consistent.
./script/release-check.sh
