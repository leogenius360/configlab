#!/bin/sh
set -eu

ROOT=$(CDPATH= cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

cargo test --locked --release -p configlab --test performance_baseline -- --ignored --nocapture
