#!/bin/sh
set -eu
ROOT=$(CDPATH= cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

cargo update
printf '%s\n' "Cargo.lock refreshed intentionally. Review it before running ./script/check.sh."
